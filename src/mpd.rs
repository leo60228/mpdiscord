use anyhow::{ensure, Result};
use serde::{de::DeserializeOwned, Deserialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;

#[derive(Deserialize)]
pub struct Song {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

pub use mparsed::Status;

pub struct SongStatus {
    pub song: Song,
    pub status: Status,
}

pub struct Mpd {
    stream: BufStream<TcpStream>,
    protocol_version: String,
}

impl Mpd {
    pub async fn new() -> Result<Self> {
        let mut stream = BufStream::new(TcpStream::connect("localhost:6600").await?);

        let mut protocol_line = String::new();
        stream.read_line(&mut protocol_line).await?;

        ensure!(
            protocol_line.starts_with("OK MPD "),
            "bad protocol version line: {:?}",
            protocol_line
        );

        let protocol_version = protocol_line["OK MPD ".len()..].trim().to_string();

        Ok(Self {
            stream,
            protocol_version,
        })
    }

    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    pub async fn raw_response(&mut self, command: &[u8]) -> Result<String> {
        self.stream.write_all(command).await?;
        self.stream.write_u8(b'\n').await?;
        self.stream.flush().await?;

        let mut response = String::new();
        while response.trim().lines().last() != Some("OK") {
            self.stream.read_line(&mut response).await?;
        }

        Ok(response)
    }

    pub async fn response<T: DeserializeOwned>(&mut self, command: &[u8]) -> Result<T> {
        let raw = self.raw_response(command).await?;
        let parsed = mparsed::parse_response(raw.lines())?;
        Ok(parsed)
    }

    pub async fn status(&mut self) -> Result<Status> {
        self.response(b"status").await
    }

    pub async fn current_song(&mut self) -> Result<Song> {
        self.response(b"currentsong").await
    }

    pub async fn idle(&mut self) -> Result<String> {
        self.raw_response(b"idle").await
    }

    pub async fn song_status(&mut self) -> Result<SongStatus> {
        let song = self.current_song().await?;
        let status = self.status().await?;

        Ok(SongStatus { song, status })
    }
}
