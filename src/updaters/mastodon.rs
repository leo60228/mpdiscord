use super::safe_recv;
use crate::config::Config;
use crate::conversions;
use crate::mastodon::Mastodon;
use crate::StatusRx;
use anyhow::Result;
use log::*;
use std::sync::Arc;

pub async fn mastodon_updater(config: Arc<Config>, mut rx: StatusRx) -> Result<!> {
    let mastodon = Mastodon::new(&config);

    let account = mastodon.account().await?;
    info!("logged in as {}", account.acct);

    loop {
        trace!("getting status");
        let song_status = safe_recv(&mut rx).await?;

        if let Some(notice) = conversions::get_text(&song_status) {
            trace!("getting mastodon account");
            let account = mastodon.account().await?;

            let bio = account
                .source
                .note
                .splitn(2, "Last listening to:")
                .next()
                .unwrap_or("")
                .trim_end();

            let new_bio = format!("{}\n\nLast listening to: {}", bio, notice);

            debug!("updating: {}", notice);
            mastodon.set_bio(&new_bio).await?;
            info!("set bio");
        } else {
            debug!("(no song)");
        }
    }
}
