#![feature(never_type)]

use anyhow::Result;
use config::Config;
use conversions::get_activity;
use discord::DiscordHandle;
use log::*;
use mpd::SongStatus;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task;

pub mod config;
pub mod conversions;
pub mod discord;
pub mod mpd;
pub mod mpd_watcher;

pub type StatusTx = broadcast::Sender<SongStatus>;
pub type StatusRx = broadcast::Receiver<SongStatus>;

async fn safe_recv(rx: &mut StatusRx) -> Result<SongStatus> {
    loop {
        match rx.recv().await {
            Ok(x) => break Ok(x),
            Err(broadcast::error::RecvError::Lagged(_)) => {
                warn!("updater lagged behind mpd");
                continue;
            }
            Err(x) => break Err(x.into()),
        }
    }
}

async fn discord_updater(
    handle: DiscordHandle,
    config: Arc<Config>,
    mut rx: StatusRx,
) -> Result<!> {
    let user = handle.user().await?;
    info!("logged in as @{}#{}", user.username(), user.discriminator());

    loop {
        trace!("getting status");
        let song_status = safe_recv(&mut rx).await?;

        let activity = get_activity(&song_status, &config)?;

        trace!("updating activity");
        handle.update_activity(activity).await?;
        info!("updated activity");
    }
}

async fn run_discord_updater(config: Arc<Config>, tx: StatusTx) -> Result<!> {
    let discord_client_id = config.discord_client_id;

    let discord_thread = discord::run_discord_thread(
        move |handle| {
            info!("connected to discord");

            let config = config.clone();
            let rx = tx.subscribe();

            let fut = async move {
                discord_updater(handle, config, rx).await.unwrap();
            };
            let fut_handle = task::spawn(fut);
            move || fut_handle.abort()
        },
        discord_client_id,
    );

    discord_thread.await
}

pub async fn run(config: Arc<Config>) -> Result<!> {
    let (tx, _rx) = broadcast::channel(2);

    let mpd_watch = mpd_watcher::mpd_watcher(tx.clone());
    let discord_thread = run_discord_updater(config.clone(), tx);

    tokio::select! {
        mpd_error = mpd_watch => mpd_error,
        discord_err = discord_thread => discord_err,
    }
}
