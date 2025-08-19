use super::config::Config;
use super::mpd::SongStatus;
use anyhow::Result;
use discord_sdk::activity::{Activity, ActivityKind, Assets, Timestamps};
use log::*;
use mpd_client::responses::{PlayState, Song};
use rand::distr::{Alphanumeric, SampleString};
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

fn get_artist(song: &Song) -> Option<String> {
    let artists = song.artists();
    let list = if !artists.is_empty() {
        artists
    } else {
        song.album_artists()
    };

    if !list.is_empty() {
        Some(list.join(", "))
    } else {
        None
    }
}

pub fn get_activity(song_status: &SongStatus, config: &Config) -> Result<Activity> {
    let time = SystemTime::now();

    trace!("creating Activity");
    let mut activity = Activity::default();

    if let Some(title) = song_status.song.as_ref().and_then(|x| x.title()) {
        debug!("{}", title);
        activity.details = Some(title.to_string());

        let album_line = song_status
            .song
            .as_ref()
            .and_then(|x| x.album())
            .map(|x| format!("(album: {x})"));

        let slug = slugify(title, config);
        if config.artfiles.contains(&slug) {
            debug!("(Cover)");
            activity.assets = Some(Assets {
                large_image: Some(slug),
                large_text: album_line,
                ..Default::default()
            });
        } else if let Some(web_config) = &config.web {
            let token = Alphanumeric.sample_string(&mut rand::rng(), 12);
            activity.assets = Some(Assets {
                large_image: Some(format!(
                    "{}/art/{}?{}",
                    web_config.public_addr,
                    song_status.status.current_song.unwrap().1 .0,
                    token
                )),
                large_text: album_line,
                ..Default::default()
            });
        }
    }

    let mut state = String::new();

    if let Some(artist) = song_status.song.as_ref().and_then(get_artist) {
        write!(state, "by {} ", artist)?;
    }

    debug!("{}", state);

    if !state.is_empty() {
        activity.state = Some(state);
    }

    if song_status.status.state == PlayState::Playing {
        if let Some(elapsed) = song_status.status.elapsed {
            debug!("Elapsed: {:?}", elapsed);

            let start = time - elapsed;
            let since_epoch = start.duration_since(UNIX_EPOCH)?;
            let end = song_status
                .status
                .duration
                .map(|duration| (duration.as_secs() + since_epoch.as_secs()) as _);
            activity.timestamps = Some(Timestamps {
                start: Some(since_epoch.as_secs() as _),
                end,
            });
        }
    }

    activity.kind = ActivityKind::Listening;

    Ok(activity)
}

pub fn get_text(song_status: &SongStatus) -> Option<String> {
    let song = song_status.song.as_ref()?;
    let title = song.title()?;
    let artist = get_artist(song).unwrap_or_else(|| "Unknown Artist".to_string());

    let mut notice = format!("{} - {}", title, artist);
    if let Some(album) = song.album() {
        write!(notice, " (album: {})", album).unwrap();
    }

    Some(notice)
}
