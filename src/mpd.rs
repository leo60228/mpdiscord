use anyhow::{anyhow, Result};
use mpd_client::client::{ConnectionEvent, ConnectionEvents, Subsystem};
use mpd_client::commands::QueueRange;
use mpd_client::Client;
use tokio::net::TcpStream;

pub use mpd_client::responses::{Song, Status};

#[derive(Clone, Debug)]
pub struct SongStatus {
    pub song: Option<Song>,
    pub status: Status,
}

pub struct Mpd {
    client: Client,
    events: ConnectionEvents,
}

impl Mpd {
    pub async fn new() -> Result<Self> {
        let stream = TcpStream::connect("localhost:6600").await?;

        let (client, events) = Client::connect(stream).await?;

        Ok(Self { client, events })
    }

    pub fn protocol_version(&self) -> &str {
        self.client.protocol_version()
    }

    pub async fn status(&mut self) -> Result<Status> {
        Ok(self.client.command(mpd_client::commands::Status).await?)
    }

    pub async fn idle(&mut self) -> Result<Subsystem> {
        match self.events.next().await {
            Some(ConnectionEvent::SubsystemChange(x)) => Ok(x),
            Some(ConnectionEvent::ConnectionClosed(err)) => Err(err.into()),
            None => Err(anyhow!("connection closed")),
        }
    }

    pub async fn song_status(&mut self) -> Result<SongStatus> {
        let status = self.status().await?;
        let song = if let Some((_, id)) = status.current_song {
            let range = self.client.command(QueueRange::song(id)).await?;
            range.into_iter().next().map(|x| x.song)
        } else {
            None
        };

        Ok(SongStatus { song, status })
    }
}
