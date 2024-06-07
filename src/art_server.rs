use anyhow::{bail, Result};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use bytes::BytesMut;
use mpd_client::commands::SongId;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::config::WebConfig;
use crate::mpd::Mpd;

async fn art(Path(song_id): Path<u64>, State(mpd): State<Arc<Mpd>>) -> impl IntoResponse {
    let song_id = SongId(song_id);

    match mpd.song_art(song_id).await {
        Ok(Some((data, mime))) => (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                mime.unwrap_or_else(|| "application/octet-stream".to_string()),
            )],
            data,
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            "Not found".into(),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            BytesMut::from(&*err.to_string()),
        ),
    }
}

pub async fn serve(web_config: &WebConfig, mpd: Arc<Mpd>) -> Result<!> {
    let app = Router::new()
        .route("/art/:song_id", get(art))
        .with_state(mpd);

    let listener = TcpListener::bind(web_config.listen_addr).await?;

    axum::serve(listener, app).await?;

    bail!("server shut down");
}
