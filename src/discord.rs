use anyhow::Result;
use crossbeam::queue::SegQueue;
use discord_game_sdk::{Activity, CreateFlags, Discord, EventHandler, User};
use log::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use tokio::task::LocalSet;

type DiscordCallback = Box<dyn FnOnce(&Discord<'static, DiscordHandle>) + Send>;

#[derive(Clone)]
pub struct DiscordHandle {
    user_tx: Arc<watch::Sender<Option<User>>>,
    user_rx: watch::Receiver<Option<User>>,
    queue: Arc<SegQueue<DiscordCallback>>,
}

pub struct Responder<O>(oneshot::Sender<O>);

impl<O> Responder<O> {
    pub fn finish(self, val: O) {
        assert!(self.0.send(val).is_ok(), "failed to send response");
    }
}

impl DiscordHandle {
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
        C: FnOnce(&Discord<'static, DiscordHandle>, Responder<O>) + Send + 'static,
        O: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.queue.push(Box::new(move |discord| {
            callback(discord, Responder(tx));
        }));

        rx.await.unwrap()
    }

    pub async fn update_activity(&self, activity: Activity) -> Result<()> {
        self.with(move |discord, resp| {
            discord.update_activity(&activity, move |_, res| {
                resp.finish(res);
            });
        })
        .await?;

        Ok(())
    }
}

impl Default for DiscordHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for DiscordHandle {
    fn on_current_user_update(&mut self, discord: &Discord<Self>) {
        self.user_tx
            .send(Some(discord.current_user().unwrap()))
            .unwrap();
    }
}

async fn run_discord<F1, F2>(on_connection: F1, client_id: i64) -> Result<!>
where
    F1: Fn(DiscordHandle) -> F2,
    F2: FnOnce(),
{
    'reconnect: loop {
        let handle = DiscordHandle::new();

        let mut client = match Discord::<DiscordHandle>::with_create_flags(
            client_id,
            CreateFlags::NoRequireDiscord,
        ) {
            Ok(x) => x,
            Err(discord_game_sdk::Error::Internal) => {
                warn!("couldn't connect, retrying...");
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
                    warn!("disconnected, reconnecting...");
                    continue 'reconnect;
                }
                Err(e) => return Err(e.into()),
            }

            while let Some(callback) = handle.queue.pop() {
                callback(&client);
            }
            tokio::task::yield_now().await;
        }
    }
}

pub async fn run_discord_thread<F1, F2>(on_connection: F1, client_id: i64) -> Result<!>
where
    F1: Fn(DiscordHandle) -> F2 + Send + 'static,
    F2: FnOnce(),
{
    let local = LocalSet::new();
    local.run_until(run_discord(on_connection, client_id)).await
}
