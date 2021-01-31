#![feature(never_type)]

use anyhow::{Context, Result};
use log::*;
use mpdiscord::{config::read_config, discord::run_discord_thread, run};
use simple_logger::SimpleLogger;
use std::env::args_os;
use tokio::task;

#[tokio::main]
async fn main() -> Result<!> {
    SimpleLogger::new().init()?;

    let config_path = args_os().nth(1).context("Missing configuration path!")?;
    let config = read_config(&config_path).await?;

    run_discord_thread(move |handle| {
        info!("connected to discord");

        let config = config.clone();

        let fut = async move {
            run(handle, config).await.unwrap();
        };
        let fut_handle = task::spawn(fut);
        move || fut_handle.abort()
    })
    .await
}
