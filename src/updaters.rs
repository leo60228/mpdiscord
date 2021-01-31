use super::mpd::SongStatus;
use super::StatusRx;
use anyhow::Result;
use log::*;
use tokio::sync::broadcast::error::RecvError;

pub mod discord;

async fn safe_recv(rx: &mut StatusRx) -> Result<SongStatus> {
    loop {
        match rx.recv().await {
            Ok(x) => break Ok(x),
            Err(RecvError::Lagged(_)) => {
                warn!("updater lagged behind mpd");
                continue;
            }
            Err(x) => break Err(x.into()),
        }
    }
}
