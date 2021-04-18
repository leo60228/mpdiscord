use super::safe_recv;
use crate::config::Config;
use crate::conversions::get_activity;
use crate::discord::{run_discord_thread, DiscordHandle};
use crate::{StatusRx, StatusTx};
use anyhow::Result;
use log::*;
use std::sync::Arc;
use tokio::task;

async fn discord_updater_inner(
    handle: DiscordHandle,
    config: Arc<Config>,
    mut rx: StatusRx,
) -> Result<!> {
    loop {
        trace!("getting status");
        let song_status = safe_recv(&mut rx).await?;

        let activity = get_activity(&song_status, &config)?;

        trace!("updating activity");
        handle.update_activity(activity).await?;
        info!("updated activity");
    }
}

pub async fn discord_updater(config: Arc<Config>, tx: StatusTx) -> Result<!> {
    let discord_client_id = config.discord_client_id;

    let discord_thread = run_discord_thread(
        move |handle| {
            info!("connected to discord");

            let config = config.clone();
            let rx = tx.subscribe();

            let fut = async move {
                discord_updater_inner(handle, config, rx).await.unwrap();
            };
            let fut_handle = task::spawn(fut);
            move || fut_handle.abort()
        },
        discord_client_id,
    );

    discord_thread.await
}
