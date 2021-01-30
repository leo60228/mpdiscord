#![feature(never_type, type_alias_impl_trait, or_patterns)]

use anyhow::{Context, Result};
use crossbeam::queue::SegQueue;
use discord_game_sdk::{Activity, CreateFlags, Discord, EventHandler, User};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use serde::Deserialize;
use std::fmt::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};

#[derive(Clone)]
pub struct EventHandlerHandle {
    user_tx: Arc<watch::Sender<Option<User>>>,
    user_rx: watch::Receiver<Option<User>>,
    queue: Arc<SegQueue<FullyErasedCallback>>,
}

trait ErasedDiscordCallback<'a> {
    fn call_once_async(
        self,
        discord: &'a Discord<'static, EventHandlerHandle>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>;
}

struct DiscordCallbackEraser<T: for<'a> DiscordCallback<'a, Output = O>, O> {
    callback: T,
    tx: oneshot::Sender<O>,
}

impl<'a, T, O> ErasedDiscordCallback<'a> for DiscordCallbackEraser<T, O>
where
    T: for<'b> DiscordCallback<'b, Output = O> + 'static,
    O: Send + 'static,
{
    fn call_once_async(
        self,
        discord: &'a Discord<'static, EventHandlerHandle>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let tx = self.tx;
        let fut = self.callback.call_once_async(discord);
        Box::pin(async move {
            assert!(tx.send(fut.await).is_ok());
        })
    }
}

trait ErasedDiscordCallbackEraser<'a> {
    fn call_box_async(
        self: Box<Self>,
        discord: &'a Discord<'static, EventHandlerHandle>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>;
}

impl<'a, T: ErasedDiscordCallback<'a>> ErasedDiscordCallbackEraser<'a> for T {
    fn call_box_async(
        self: Box<Self>,
        discord: &'a Discord<'static, EventHandlerHandle>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        self.call_once_async(discord)
    }
}

type FullyErasedCallback = Box<dyn for<'a> ErasedDiscordCallbackEraser<'a> + Send + Sync>;

pub trait DiscordCallback<'a> {
    type Future: Future<Output = Self::Output> + 'a;
    type Output: Send + 'static;

    fn call_once_async(self, discord: &'a Discord<'static, EventHandlerHandle>) -> Self::Future;
}

impl<'a, F, Fut> DiscordCallback<'a> for F
where
    F: FnOnce(&'a Discord<'static, EventHandlerHandle>) -> Fut,
    Fut: Future + 'a,
    Fut::Output: Send + 'static,
{
    type Future = Fut;
    type Output = Fut::Output;

    fn call_once_async(self, discord: &'a Discord<'static, EventHandlerHandle>) -> Self::Future {
        self(discord)
    }
}

impl EventHandlerHandle {
    pub fn new() -> Self {
        let (user_tx, user_rx) = watch::channel(None);
        Self {
            user_tx: Arc::new(user_tx),
            user_rx,
            queue: Arc::new(SegQueue::new()),
        }
    }

    pub async fn user(&self) -> Result<User> {
        let mut rx = self.user_rx.clone();

        loop {
            if let Some(user) = rx.borrow().clone() {
                return Ok(user);
            }

            rx.changed().await?;
        }
    }

    pub async fn with<C, O>(&self, callback: C) -> O
    where
        C: for<'a> DiscordCallback<'a, Output = O> + Send + Sync + 'static,
        O: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.queue
            .push(Box::new(DiscordCallbackEraser { callback, tx }));
        rx.await.unwrap()
    }

    pub async fn update_activity(&self, activity: Activity) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        self.with(with_closure!(move |discord| -> () {
            discord.update_activity(&activity, |_, res| {
                tx.send(res).unwrap();
            });
        }))
        .await;

        rx.await.unwrap()?;

        Ok(())
    }
}

impl Default for EventHandlerHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for EventHandlerHandle {
    fn on_current_user_update(&mut self, discord: &Discord<Self>) {
        self.user_tx
            .send(Some(discord.current_user().unwrap()))
            .unwrap();
    }
}

const CLIENT_ID: i64 = 804763079581761556;

async fn run_discord<F1, F2>(on_connection: F1) -> Result<!>
where
    F1: Fn(EventHandlerHandle) -> F2,
    F2: FnOnce(),
{
    'reconnect: loop {
        let handle = EventHandlerHandle::new();

        let mut client = match Discord::<EventHandlerHandle>::with_create_flags(
            CLIENT_ID,
            CreateFlags::NoRequireDiscord,
        ) {
            Ok(x) => x,
            Err(discord_game_sdk::Error::Internal) => {
                println!("couldn't connect, retrying...");
                tokio::time::sleep(Duration::from_millis(1000)).await;
                continue 'reconnect;
            }
            Err(e) => return Err(e.into()),
        };
        *client.event_handler_mut() = Some(handle.clone());

        let _disconnection = finally_block::finally(on_connection(handle.clone()));

        loop {
            match client.run_callbacks() {
                Ok(()) => (),
                Err(discord_game_sdk::Error::NotRunning) => {
                    println!("disconnected, reconnecting...");
                    continue 'reconnect;
                }
                Err(e) => return Err(e.into()),
            }

            let futures = FuturesUnordered::new();
            while let Some(callback) = handle.queue.pop() {
                futures.push(callback.call_box_async(&client));
            }
            futures.collect::<()>().await;
            tokio::task::yield_now().await;
        }
    }
}

fn run_discord_thread<F1, F2>(on_connection: F1) -> impl Future<Output = Result<!>>
where
    F1: Fn(EventHandlerHandle) -> F2 + Send + 'static,
    F2: FnOnce(),
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let thread = async_thread::spawn(move || {
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, run_discord(on_connection))
    });

    async move {
        match thread.join().await {
            Ok(x) => x,
            Err(x) => std::panic::resume_unwind(x),
        }
    }
}

#[macro_export]
macro_rules! with_closure {
    ($($move:ident)? |$discord:pat| -> $res:ty { $($body:tt)* }) => {
        {
            type CallbackFut<'a> = impl Future<Output = $res>;

            fn dummy<F>(f: F) -> F
            where
                F: for<'a> FnOnce(&'a Discord<'static, EventHandlerHandle>) -> CallbackFut<'a>,
            {
                f
            }

            dummy($($move)? |$discord: &Discord<'static, EventHandlerHandle>| async move { $($body)* })
        }
    }
}

async fn run(handle: EventHandlerHandle) -> Result<!> {
    let user = handle.user().await?;
    println!("logged in as {:#?}", user);

    let mut stream = BufStream::new(TcpStream::connect("localhost:6600").await?);
    let artfiles_path = std::env::args_os().nth(1).context("missing path")?;
    let artfiles = tokio::fs::read_to_string(artfiles_path).await?;

    #[derive(Deserialize)]
    struct Song {
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
    }

    let mut connect_resp = String::new();
    stream.read_line(&mut connect_resp).await?;
    print!("connected, {}", connect_resp);

    loop {
        let time = SystemTime::now();
        println!("getting status");
        stream.write_all(b"status\n").await?;
        stream.flush().await?;
        let mut status_resp = String::new();
        while status_resp.trim().lines().last() != Some("OK") {
            stream.read_line(&mut status_resp).await?;
        }
        let status: mparsed::Status = mparsed::parse_response(status_resp.lines())?;

        println!("getting song");
        stream.write_all(b"currentsong\n").await?;
        stream.flush().await?;
        let mut song_resp = String::new();
        while song_resp.trim().lines().last() != Some("OK") {
            stream.read_line(&mut song_resp).await?;
        }
        let song: Song = mparsed::parse_response(song_resp.lines())?;
        println!("got song");

        let mut activity = Activity::empty();

        if let Some(title) = &song.title {
            println!("{}", title);
            activity.with_details(title);

            let slug: String = title
                .chars()
                .scan(false, |state, x| {
                    if x.is_ascii_alphabetic() {
                        *state = false;
                        Some(Some(x.to_ascii_lowercase()))
                    } else if *state {
                        Some(None)
                    } else {
                        *state = true;
                        Some(Some('-'))
                    }
                })
                .flatten()
                .collect();
            if artfiles.lines().any(|x| x == slug) {
                println!("(Cover)");
                activity.with_large_image_key(&slug);
                activity.with_large_image_tooltip(&title);
            }
        }

        let mut state = String::new();

        if let Some(artist) = &song.artist {
            write!(state, "by {} ", artist)?;
        }

        if let Some(album) = &song.album {
            write!(state, "(album: {})", album)?;
        }

        println!("{}", state);

        activity.with_state(&state);

        if status.state == mparsed::State::Play {
            if let Some(elapsed) = status.elapsed {
                println!("Elapsed: {:?}", elapsed);

                let start = time - elapsed;
                let since_epoch = start.duration_since(UNIX_EPOCH)?;
                activity.with_start_time(since_epoch.as_secs() as _);
            }
        }

        handle.update_activity(activity).await?;

        stream.write_all(b"idle\n").await?;
        stream.flush().await?;
        let mut idle_resp = String::new();
        while idle_resp.trim().lines().last() != Some("OK") {
            stream.read_line(&mut idle_resp).await?;
        }
    }
}

fn main() -> Result<!> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let rt_handle = rt.handle().clone();

        let discord = run_discord_thread(move |handle| {
            println!("connected");

            let (fut, fut_handle) = future::abortable(async move {
                run(handle).await.unwrap();
            });
            rt_handle.spawn(fut);
            move || fut_handle.abort()
        });

        discord.await
    })
}
