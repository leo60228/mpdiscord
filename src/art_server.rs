use anyhow::{bail, Result};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{ErrorResponse, IntoResponse};
use axum::routing::get;
use axum::Router;
use bytes::BytesMut;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::ImageFormat;
use mpd_client::commands::SongId;
use std::fmt::Display;
use std::io::Cursor;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::config::WebConfig;
use crate::mpd::Mpd;

fn err(x: impl Display) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, x.to_string())
}

async fn art(
    Path(song_id): Path<u64>,
    State(mpd): State<Arc<Mpd>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let song_id = SongId(song_id);

    match mpd.song_art(song_id).await {
        Ok(Some((data, mime))) => {
            let mut reader = ImageReader::new(Cursor::new(&data));

            if let Some(format) = mime.and_then(ImageFormat::from_mime_type) {
                reader.set_format(format);
            } else {
                reader = reader.with_guessed_format().map_err(err)?;
            }

            if let Some(format) = reader.format() {
                let image = reader.decode().map_err(err)?;

                let mut headers = HeaderMap::new();
                let mime = format.to_mime_type().parse().map_err(err)?;
                headers.insert(header::CONTENT_TYPE, mime);

                if image.width() > 1024 || image.height() > 1024 {
                    let resized = image.resize(1000, 1000, FilterType::CatmullRom);
                    let mut writer = Cursor::new(vec![]);
                    resized.write_to(&mut writer, format).map_err(err)?;
                    let data = BytesMut::from(&*writer.into_inner());
                    Ok((StatusCode::OK, headers, data))
                } else {
                    Ok((StatusCode::OK, headers, data))
                }
            } else {
                Ok((StatusCode::OK, HeaderMap::new(), data))
            }
        }
        Ok(None) => Ok((StatusCode::NOT_FOUND, HeaderMap::new(), "Not found".into())),
        Err(err) => Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            HeaderMap::new(),
            BytesMut::from(&*err.to_string()),
        )),
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
