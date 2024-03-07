use super::safe_recv;
use crate::config::Config;
use crate::conversions::get_activity;
use crate::discord::DiscordHandle;
use crate::StatusRx;
use anyhow::Result;
use log::*;
use std::sync::Arc;

pub async fn discord_updater(config: Arc<Config>, mut rx: StatusRx) -> Result<!> {
    let mut handle = DiscordHandle::new(config.discord_client_id);

    loop {
        trace!("getting status");
        let song_status = safe_recv(&mut rx).await?;

        let activity = get_activity(&song_status, &config)?;

        trace!("updating activity");
        handle.update_activity(activity).await?;
        info!("updated activity");
    }
}
