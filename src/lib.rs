#![feature(never_type, type_alias_impl_trait, or_patterns)]

use anyhow::{Context, Result};
use discord::EventHandlerHandle;
use discord_game_sdk::Activity;
use serde::Deserialize;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;

pub mod discord;

pub async fn run(handle: EventHandlerHandle) -> Result<!> {
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
