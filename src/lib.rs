#![feature(never_type)]

use anyhow::Result;
use config::Config;
use conversions::get_activity;
use discord::DiscordHandle;
use log::*;
use mpd::Mpd;
use std::sync::Arc;

pub mod config;
mod conversions;
pub mod discord;
pub mod mpd;

pub async fn run(handle: DiscordHandle, config: Arc<Config>) -> Result<!> {
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
