#![feature(never_type)]

use anyhow::{Context, Result};
use discord::DiscordHandle;
use discord_game_sdk::Activity;
use mpd::Mpd;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub async fn run(handle: DiscordHandle) -> Result<!> {
    let user = handle.user().await?;
    println!("logged in as {:#?}", user);

    let artfiles_path = std::env::args_os().nth(1).context("missing path")?;
    let artfiles = tokio::fs::read_to_string(artfiles_path).await?;

    let mut mpd = Mpd::new().await?;

    print!("connected to mpd {}", mpd.protocol_version());

    loop {
        let time = SystemTime::now();

        let status = mpd.status().await?;
        let song = mpd.current_song().await?;

        let mut activity = Activity::empty();

        if let Some(title) = &song.title {
            println!("{}", title);
            activity.with_details(title);

            let slug = slugify(&title);
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

        mpd.idle().await?;
    }
}
