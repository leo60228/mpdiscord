#![feature(never_type)]

use anyhow::Result;
use config::Config;
use discord::DiscordHandle;
use discord_game_sdk::Activity;
use log::*;
use mpd::Mpd;
use std::fmt::Write;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod config;
pub mod discord;
pub mod mpd;

fn slugify(title: &str) -> String {
    title
        .chars()
        .scan(false, |state, x| {
            if x.is_ascii_alphanumeric() {
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
        .take(16)
        .collect()
}

pub async fn run(handle: DiscordHandle, config: Arc<Config>) -> Result<!> {
    let user = handle.user().await?;
    info!("logged in as @{}#{}", user.username(), user.discriminator());

    trace!("connecting to mpd");
    let mut mpd = Mpd::new().await?;

    info!("connected to mpd {}", mpd.protocol_version());

    loop {
        let time = SystemTime::now();

        trace!("getting status");
        let status = mpd.status().await?;
        trace!("getting song");
        let song = mpd.current_song().await?;

        trace!("setting up activity");

        let mut activity = Activity::empty();

        if let Some(title) = &song.title {
            debug!("{}", title);
            activity.with_details(title);

            let slug = slugify(&title);
            if config.artfiles.contains(&slug) {
                debug!("(Cover)");
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

        debug!("{}", state);

        activity.with_state(&state);

        if status.state == mparsed::State::Play {
            if let Some(elapsed) = status.elapsed {
                debug!("Elapsed: {:?}", elapsed);

                let start = time - elapsed;
                let since_epoch = start.duration_since(UNIX_EPOCH)?;
                activity.with_start_time(since_epoch.as_secs() as _);
            }
        }

        trace!("updating activity");

        handle.update_activity(activity).await?;

        info!("updated activity, idling");

        mpd.idle().await?;
    }
}
