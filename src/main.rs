#![feature(never_type)]

use anyhow::Result;
use futures::prelude::*;
use mpdiscord::{discord::run_discord_thread, run};

fn main() -> Result<!> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let rt_handle = rt.handle().clone();

        let discord = run_discord_thread(move |handle| {
            println!("connected");

            let (fut, fut_handle) = future::abortable(async move {
                run(handle).await.unwrap();
            });
            rt_handle.spawn(fut);
            move || fut_handle.abort()
        });

        discord.await
    })
}
