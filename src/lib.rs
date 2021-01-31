#![feature(never_type)]

use anyhow::Result;
use config::Config;
use conversions::get_activity;
use discord::DiscordHandle;
use log::*;
use mpd::Mpd;
use std::sync::Arc;
use tokio::task;

pub mod config;
mod conversions;
pub mod discord;
pub mod mpd;

pub async fn run_discord(handle: DiscordHandle, config: Arc<Config>) -> Result<!> {
    let user = handle.user().await?;
    info!("logged in as @{}#{}", user.username(), user.discriminator());

    trace!("connecting to mpd");
    let mut mpd = Mpd::new().await?;

    info!("connected to mpd {}", mpd.protocol_version());

    loop {
        trace!("getting status");
        let song_status = mpd.song_status().await?;

        let activity = get_activity(&song_status, &config)?;

        trace!("updating activity");

        handle.update_activity(activity).await?;

        info!("updated activity, idling");

        mpd.idle().await?;
    }
}

pub async fn run(config: Arc<Config>) -> Result<!> {
    let discord_client_id = config.discord_client_id;

    discord::run_discord_thread(
        move |handle| {
            info!("connected to discord");

            let config = config.clone();

            let fut = async move {
                run_discord(handle, config).await.unwrap();
            };
            let fut_handle = task::spawn(fut);
            move || fut_handle.abort()
        },
        discord_client_id,
    )
    .await
}
