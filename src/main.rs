#![feature(never_type)]

use anyhow::Result;
use discord_game_sdk::{Discord, EventHandler, User};
use once_cell::sync::OnceCell;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventHandlerHandle {
    user_update_tx: broadcast::Sender<User>,
    user: Arc<OnceCell<User>>,
}

impl EventHandlerHandle {
    pub fn new() -> Self {
        let (user_update_tx, _) = broadcast::channel(1);
        Self {
            user_update_tx,
            user: Arc::new(OnceCell::new()),
        }
    }

    pub async fn user(&self) -> Result<User> {
        if let Some(user) = self.user.get() {
            Ok(user.clone())
        } else {
            Ok(self.user_update_tx.subscribe().recv().await?)
        }
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

#[tokio::main]
async fn main() -> Result<!> {
    let handle = EventHandlerHandle::new();

    let discord = run_discord_thread(handle.clone());

    let user = handle.user().await?;
    println!("connected as {:#?}", user);

    discord.await?
}
