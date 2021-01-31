use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub artfiles: Vec<String>,
}

pub async fn read_config(path: impl AsRef<Path>) -> Result<Arc<Config>> {
    let config_text = fs::read(path).await?;
    let parsed = toml::from_slice(&config_text)?;
    Ok(Arc::new(parsed))
}