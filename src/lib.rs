#![feature(never_type)]

use anyhow::Result;
use config::Config;
use mpd::SongStatus;
use std::sync::Arc;
use tokio::sync::broadcast;

pub mod config;
pub mod conversions;
pub mod discord;
pub mod mastodon;
pub mod mpd;
pub mod mpd_watcher;
pub mod updaters;

pub type StatusTx = broadcast::Sender<SongStatus>;
pub type StatusRx = broadcast::Receiver<SongStatus>;

pub async fn run(config: Arc<Config>) -> Result<!> {
    let (tx, rx) = broadcast::channel(2);

    let mpd_watch = mpd_watcher::mpd_watcher(tx.clone());
    let discord_thread = updaters::discord::discord_updater(config.clone(), rx);
    let mastodon = updaters::mastodon::mastodon_updater(config.clone(), tx.subscribe());

    tokio::select! {
        mpd_error = mpd_watch => mpd_error,
        discord_err = discord_thread => discord_err,
        mastodon_err = mastodon => mastodon_err,
    }
}
