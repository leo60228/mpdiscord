#![feature(never_type)]

use anyhow::Result;
use log::*;
use mpdiscord::{discord::run_discord_thread, run};
use simple_logger::SimpleLogger;

fn main() -> Result<!> {
    SimpleLogger::new().init()?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let rt_handle = rt.handle().clone();

        let discord = run_discord_thread(move |handle| {
            info!("connected to discord");

            let fut = async move {
                run(handle).await.unwrap();
            };
            let fut_handle = rt_handle.spawn(fut);
            move || fut_handle.abort()
        });

        discord.await
    })
}
