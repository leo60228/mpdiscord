use super::config::Config;
use super::mpd::SongStatus;
use anyhow::Result;
use discord_game_sdk::Activity;
use log::*;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn get_activity(song_status: &SongStatus, config: &Config) -> Result<Activity> {
    let time = SystemTime::now();

    trace!("creating Activity");
    let mut activity = Activity::empty();

    if let Some(title) = &song_status.song.title {
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

    if let Some(artist) = &song_status.song.artist {
        write!(state, "by {} ", artist)?;
    }

    if let Some(album) = &song_status.song.album {
        write!(state, "(album: {})", album)?;
    }

    debug!("{}", state);

    activity.with_state(&state);

    if song_status.status.state == mparsed::State::Play {
        if let Some(elapsed) = song_status.status.elapsed {
            debug!("Elapsed: {:?}", elapsed);

            let start = time - elapsed;
            let since_epoch = start.duration_since(UNIX_EPOCH)?;
            activity.with_start_time(since_epoch.as_secs() as _);
        }
    }

    Ok(activity)
}

pub fn get_text(song_status: &SongStatus) -> Option<String> {
    if let (Some(title), Some(artist)) = (&song_status.song.title, &song_status.song.artist) {
        let mut notice = format!("{} - {}", title, artist);
        if let Some(album) = &song_status.song.album {
            write!(notice, " (album: {})", album).unwrap();
        }

        Some(notice)
    } else {
        None
    }
}
