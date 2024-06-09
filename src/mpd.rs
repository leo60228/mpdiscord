use anyhow::{anyhow, bail, Result};
use bytes::BytesMut;
use mpd_client::client::{CommandError, ConnectionEvent, ConnectionEvents, Subsystem};
use mpd_client::commands::{QueueRange, SetBinaryLimit, SongId};
pub use mpd_client::responses::{Song, Status};
use mpd_client::Client;
use tokio::net::TcpStream;

#[derive(Clone, Debug)]
pub struct SongStatus {
    pub song: Option<Song>,
    pub status: Status,
}

pub struct Mpd {
    client: Client,
}

impl Mpd {
    pub async fn connect() -> Result<(Self, ConnectionEvents)> {
        let stream = TcpStream::connect("localhost:6600").await?;

        let (client, events) = Client::connect(stream).await?;

        client.command(SetBinaryLimit(5 * 1024 * 1024)).await?;

        Ok((Self { client }, events))
    }

    pub fn protocol_version(&self) -> &str {
        self.client.protocol_version()
    }

    pub async fn status(&self) -> Result<Status> {
        Ok(self.client.command(mpd_client::commands::Status).await?)
    }

    pub async fn song_status(&self) -> Result<SongStatus> {
        let status = self.status().await?;
        let song = if let Some((_, id)) = status.current_song {
            let range = self.client.command(QueueRange::song(id)).await?;
            range.into_iter().next().map(|x| x.song)
        } else {
            None
        };

        Ok(SongStatus { song, status })
    }

    pub async fn song_art(&self, id: SongId) -> Result<Option<(BytesMut, Option<String>)>> {
        let range = match self.client.command(QueueRange::song(id)).await {
            Ok(x) => x,
            Err(CommandError::ErrorResponse {
                error: mpd_client::protocol::response::Error { code: 50, .. },
                ..
            }) => return Ok(None),
            Err(err) => bail!(err),
        };
        let song = if let Some(x) = range.into_iter().next() {
            x
        } else {
            return Ok(None);
        };
        let uri = song.song.url;

        Ok(self.client.album_art(&uri).await?)
    }
}

pub async fn idle(events: &mut ConnectionEvents) -> Result<Subsystem> {
    match events.next().await {
        Some(ConnectionEvent::SubsystemChange(x)) => Ok(x),
        Some(ConnectionEvent::ConnectionClosed(err)) => Err(err.into()),
        None => Err(anyhow!("connection closed")),
    }
}
