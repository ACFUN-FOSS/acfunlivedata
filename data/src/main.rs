mod config;
mod interval;
mod live;
mod socket;
mod sql;
mod sqlite;

use acfunlivedata_common::config::Config as CommonConfig;
use anyhow::{bail, Result};
use rpassword::read_password_from_tty;
use tokio::sync::mpsc;

const WORKER_THREAD_NUM: usize = 10;
const MAX_BLOCKING_THREAD: usize = 2048;

fn main() -> Result<()> {
    env_logger::builder()
        .filter(Some("acfunliveapi"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedanmaku"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata_common"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata"), log::LevelFilter::Trace)
        .init();

    let password = read_password_from_tty(Some("data center password: "))?;
    if password.is_empty() {
        bail!("password is empty");
    }

    let (live_tx, live_rx) = mpsc::unbounded_channel();
    live::LIVE_TX.set(live_tx).expect("failed to set LIVE_TX");
    let (all_lives_tx, all_lives_rx) = mpsc::unbounded_channel();
    live::ALL_LIVES_TX
        .set(all_lives_tx)
        .expect("failed to set ALL_LIVE_TX");
    let (gift_tx, gift_rx) = mpsc::unbounded_channel();
    live::GIFT_TX.set(gift_tx).expect("failed to set GIFT_TX");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREAD_NUM)
        .thread_name("acfunlivedata worker")
        .enable_all()
        .max_blocking_threads(MAX_BLOCKING_THREAD)
        .build()?
        .block_on(async {
            let config: config::LiveConfig = CommonConfig::new_or_load_config(
                password.clone(),
                crate::config::CONFIG_FILE_PATH.as_path(),
            )
            .await
            .expect("failed to load config");
            sqlite::create_db_dir()
                .await
                .expect("failed to create database directory");

            tokio::task::spawn_blocking(|| sqlite::all_lives(all_lives_rx));
            tokio::task::spawn_blocking(|| sqlite::gift_info(gift_rx));

            tokio::select! {
                _ = socket::message(password) => {}
                _ = interval::send_tick() => {}
                _ = live::all_lives() => {}
                _ = live::all_danmaku(live_rx, config) => {}
            }
        });

    Ok(())
}
