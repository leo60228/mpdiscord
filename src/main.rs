#![feature(never_type, type_alias_impl_trait)]

use anyhow::Result;
use crossbeam::queue::SegQueue;
use discord_game_sdk::{Activity, Discord, EventHandler, User};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use once_cell::sync::OnceCell;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};

#[derive(Clone)]
pub struct EventHandlerHandle {
    user_update_tx: broadcast::Sender<User>,
    user: Arc<OnceCell<User>>,
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
        let (user_update_tx, _) = broadcast::channel(1);
        Self {
            user_update_tx,
            user: Arc::new(OnceCell::new()),
            queue: Arc::new(SegQueue::new()),
        }
    }

    pub async fn user(&self) -> Result<User> {
        if let Some(user) = self.user.get() {
            Ok(user.clone())
        } else {
            Ok(self.user_update_tx.subscribe().recv().await?)
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
        self.user
            .get_or_try_init::<_, anyhow::Error>(|| {
                let user = discord.current_user()?;
                self.user_update_tx.send(user.clone())?;
                Ok(user)
            })
            .unwrap();
    }
}

const CLIENT_ID: i64 = 804763079581761556;

async fn run_discord(handle: EventHandlerHandle) -> Result<!> {
    let mut client = Discord::<EventHandlerHandle>::new(CLIENT_ID)?;
    *client.event_handler_mut() = Some(handle.clone());

    loop {
        client.run_callbacks()?;
        let futures = FuturesUnordered::new();
        while let Some(callback) = handle.queue.pop() {
            futures.push(callback.call_box_async(&client));
        }
        futures.collect::<()>().await;
        tokio::task::yield_now().await;
    }
}

fn run_discord_thread(handle: EventHandlerHandle) -> impl Future<Output = Result<!>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let thread = async_thread::spawn(move || {
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, run_discord(handle.clone()))
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

#[tokio::main]
async fn main() -> Result<!> {
    let handle = EventHandlerHandle::new();

    let discord = run_discord_thread(handle.clone());

    let user = handle.user().await?;
    println!("connected as {:#?}", user);

    handle.update_activity(Activity::empty()).await?;

    discord.await?
}
