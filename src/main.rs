#![feature(never_type)]

use anyhow::{Context, Result};
use mpdiscord::{config::read_config, run};
use simple_logger::SimpleLogger;
use std::env::args_os;

#[tokio::main]
async fn main() -> Result<!> {
    SimpleLogger::new().init()?;

    let config_path = args_os().nth(1).context("Missing configuration path!")?;
    let config = read_config(&config_path).await?;

    run(config).await
}
