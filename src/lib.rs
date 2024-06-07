#![feature(never_type)]

use crate::mpd::Mpd;
use anyhow::Result;
use config::Config;
use log::{info, trace};
use mpd::SongStatus;
use std::future::pending;
use std::sync::Arc;
use tokio::sync::broadcast;

pub mod art_server;
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

    trace!("connecting to mpd");
    let (mpd, events) = Mpd::connect().await?;

    let mpd = Arc::new(mpd);

    info!("connected to mpd {}", mpd.protocol_version());

    let mpd_watch = mpd_watcher::mpd_watcher(&mpd, events, tx.clone());
    let discord_thread = updaters::discord::discord_updater(config.clone(), rx);
    let mastodon = updaters::mastodon::mastodon_updater(config.clone(), tx.subscribe());
    let art_server = async {
        if let Some(web_config) = &config.web {
            art_server::serve(web_config, mpd.clone()).await
        } else {
            pending().await
        }
    };

    tokio::select! {
        mpd_error = mpd_watch => mpd_error,
        discord_err = discord_thread => discord_err,
        mastodon_err = mastodon => mastodon_err,
        art_server_err = art_server => art_server_err,
    }
}
