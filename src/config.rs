use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub artfiles: Vec<String>,
    pub discord_client_id: i64,
    pub mastodon_token: String,

    #[serde(default)]
    pub art_overrides: HashMap<String, String>,
}

pub async fn read_config(path: impl AsRef<Path>) -> Result<Arc<Config>> {
    let config_text = fs::read_to_string(path).await?;
    let parsed = toml::from_str(&config_text)?;
    Ok(Arc::new(parsed))
}
