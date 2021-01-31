#![feature(never_type)]

use anyhow::Result;
use log::*;
use mpdiscord::{discord::run_discord_thread, run};
use simple_logger::SimpleLogger;
use tokio::task;

#[tokio::main]
async fn main() -> Result<!> {
    SimpleLogger::new().init()?;

    run_discord_thread(move |handle| {
        info!("connected to discord");

        let fut = async move {
            run(handle).await.unwrap();
        };
        let fut_handle = task::spawn(fut);
        move || fut_handle.abort()
    })
    .await
}
