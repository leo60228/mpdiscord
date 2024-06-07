use super::mpd::Mpd;
use super::{mpd, StatusTx};
use anyhow::Result;
use log::*;
use mpd_client::client::ConnectionEvents;

pub async fn mpd_watcher(mpd: &Mpd, mut events: ConnectionEvents, tx: StatusTx) -> Result<!> {
    loop {
        trace!("getting status");
        let song_status = mpd.song_status().await?;

        trace!("sending status");
        tx.send(song_status)?;

        info!("sent status, idling");
        mpd::idle(&mut events).await?;
    }
}
