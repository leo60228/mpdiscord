use super::mpd::Mpd;
use super::StatusTx;
use anyhow::Result;
use log::*;

pub async fn mpd_watcher(tx: StatusTx) -> Result<!> {
    trace!("connecting to mpd");
    let mut mpd = Mpd::new().await?;

    info!("connected to mpd {}", mpd.protocol_version());

    loop {
        trace!("getting status");
        let song_status = mpd.song_status().await?;

        trace!("sending status");
        tx.send(song_status)?;

        info!("sent status, idling");
        mpd.idle().await?;
    }
}
