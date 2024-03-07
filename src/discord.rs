use anyhow::{bail, Result};
use discord_sdk::{
    activity::{Activity, ActivityArgs},
    user::{events::ConnectEvent, User},
    Discord, DiscordHandler, DiscordMsg, Event, Subscriptions,
};
use log::*;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::oneshot::{channel, Sender};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

struct EventHandler {
    sender: Mutex<Option<Sender<Result<User>>>>,
    state: Arc<RwLock<State>>,
}

#[async_trait::async_trait]
impl DiscordHandler for EventHandler {
    async fn on_message(&self, msg: DiscordMsg) {
        match msg {
            DiscordMsg::Event(Event::Ready(ConnectEvent { user, .. })) => {
                trace!("ready");

                let mut sender = self.sender.lock().await;

                if let Some(sender) = sender.take() {
                    trace!("took sender");
                    if sender.send(Ok(user)).is_err() {
                        warn!("couldn't send user");
                    }
                }
            }
            DiscordMsg::Event(Event::Disconnected { reason }) => {
                info!("disconnected");

                let mut sender = self.sender.lock().await;

                if let Some(sender) = sender.take() {
                    warn!("disconnected while connecting");
                    if sender.send(Err(reason.into())).is_err() {
                        warn!("couldn't send error");
                    }
                } else {
                    *self.state.write().await = State::Disconnected;
                }
            }
            DiscordMsg::Event(event) => {
                trace!("event: {:?}", event);
            }
            DiscordMsg::Error(err) => {
                error!("discord error: {}", err);

                let mut sender = self.sender.lock().await;

                if let Some(sender) = sender.take() {
                    trace!("took sender");
                    if sender.send(Err(err.into())).is_err() {
                        warn!("couldn't send error");
                    }
                } else {
                    *self.state.write().await = State::Error(Some(err));
                }
            }
        }
    }
}

struct Connection {
    pub discord: Discord,
    pub user: User,
}

enum State {
    Disconnected,
    Connected(Connection),
    Error(Option<discord_sdk::Error>),
}

pub struct DiscordHandle {
    client_id: i64,
    state: Arc<RwLock<State>>,
}

impl DiscordHandle {
    pub fn new(client_id: i64) -> Self {
        Self {
            client_id,
            state: Arc::new(RwLock::new(State::Disconnected)),
        }
    }

    async fn connect(&self) -> Result<impl Deref<Target = Connection> + '_> {
        let current_state = self.state.read().await;

        if let Ok(conn) = RwLockReadGuard::try_map(current_state, |x| {
            if let State::Connected(conn) = x {
                Some(conn)
            } else {
                None
            }
        }) {
            debug!("already connected");
            return Ok(conn);
        }

        let mut writer = self.state.write().await;

        if let State::Error(err) = &mut *writer {
            if let Some(err) = err.take() {
                return Err(err.into());
            } else {
                bail!("Error already taken!");
            }
        }

        debug!("connecting");

        let (sender, receiver) = channel();
        let handler = EventHandler {
            sender: Mutex::new(Some(sender)),
            state: self.state.clone(),
        };
        let discord = Discord::new(self.client_id, Subscriptions::USER, Box::new(handler))?;

        let user = receiver.await??;

        info!("logged in as {}", user.username);

        *writer = State::Connected(Connection { discord, user });

        let new_state = writer.downgrade();

        Ok(RwLockReadGuard::map(new_state, |x| {
            if let State::Connected(conn) = x {
                conn
            } else {
                unreachable!()
            }
        }))
    }

    pub async fn user(&mut self) -> Result<User> {
        let Connection { user, .. } = &*self.connect().await?;
        Ok(user.clone())
    }

    pub async fn update_activity(&mut self, activity: Activity) -> Result<()> {
        debug!("updating activity");

        let Connection { discord, .. } = &*self.connect().await?;

        let mut args = ActivityArgs::default();
        args.activity = Some(activity);
        discord.update_activity(args).await?;

        Ok(())
    }
}
