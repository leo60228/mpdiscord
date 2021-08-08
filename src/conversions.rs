use super::config::Config;
use super::mpd::SongStatus;
use anyhow::Result;
use discord_sdk::activity::{Activity, Assets, Timestamps};
use log::*;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

fn slugify(title: &str, config: &Config) -> String {
    if let Some(slug) = config.art_overrides.get(title) {
        return slug.clone();
    }

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
    let mut activity = Activity::default();

    if let Some(title) = &song_status.song.title {
        debug!("{}", title);
        activity.details = Some(title.to_string());

        let slug = slugify(title, config);
        if config.artfiles.contains(&slug) {
            debug!("(Cover)");
            activity.assets = Some(Assets {
                large_image: Some(slug),
                large_text: Some(title.to_string()),
                ..Default::default()
            });
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

    if !state.is_empty() {
        activity.state = Some(state);
    }

    if song_status.status.state == mparsed::State::Play {
        if let Some(elapsed) = song_status.status.elapsed {
            debug!("Elapsed: {:?}", elapsed);

            let start = time - elapsed;
            let since_epoch = start.duration_since(UNIX_EPOCH)?;
            activity.timestamps = Some(Timestamps {
                start: Some(since_epoch.as_secs() as _),
                end: None,
            });
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
