use super::safe_recv;
use crate::config::Config;
use crate::StatusRx;
use anyhow::Result;
use log::*;
use reqwest::Client;
use serde::Deserialize;
use std::fmt::Write;
use std::sync::Arc;

#[derive(Deserialize)]
struct AccountSource {
    pub note: String,
}

#[derive(Deserialize)]
struct OwnAccount {
    pub acct: String,
    pub source: AccountSource,
}

pub async fn mastodon_updater(config: Arc<Config>, mut rx: StatusRx) -> Result<!> {
    let token = &config.mastodon_token;

    let client = Client::new();

    let account: OwnAccount = client
        .get("https://60228.dev/api/v1/accounts/verify_credentials")
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    info!("logged in as {}", account.acct);

    loop {
        trace!("getting status");
        let song_status = safe_recv(&mut rx).await?;

        if let (Some(title), Some(artist)) = (&song_status.song.title, &song_status.song.artist) {
            trace!("getting mastodon account");
            let account: OwnAccount = client
                .get("https://60228.dev/api/v1/accounts/verify_credentials")
                .bearer_auth(token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let bio = account
                .source
                .note
                .rsplitn(2, "\n\nLast listening to:")
                .last()
                .unwrap_or("");

            let mut notice = format!("{} - {}", title, artist);
            if let Some(album) = &song_status.song.album {
                write!(notice, " (album: {})", album)?;
            }

            let new_bio = format!("{}\n\nLast listening to: {}", bio, notice);

            debug!("updating: {}", notice);
            client
                .patch("https://60228.dev/api/v1/accounts/update_credentials")
                .bearer_auth(token)
                .form(&[("note", new_bio)])
                .send()
                .await?
                .error_for_status()?;
        } else {
            debug!("(no song)");
        }
    }
}
