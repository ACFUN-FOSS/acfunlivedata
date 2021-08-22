mod auth;
mod config;
mod download;
mod model;
mod pool;
mod server;
mod socket;
mod sql;
mod sqlite;

use acfunlivedata_common::{
    config::Config as CommonConfig,
    create_dir,
    message::{MessageSocket, DATA_CENTER_SOCKET},
};
use anyhow::{bail, Result};
use rpassword::read_password_from_tty;
use std::sync::Arc;
use tokio::sync::Mutex;

const WORKER_THREAD_NUM: usize = 10;
const MAX_BLOCKING_THREAD: usize = 2048;

fn main() -> Result<()> {
    env_logger::builder()
        .filter(Some("acfunliveapi"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata_common"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata_backend"), log::LevelFilter::Trace)
        .filter(Some("async-graphql"), log::LevelFilter::Trace)
        .init();

    let data_password = read_password_from_tty(Some("data center password: "))?;
    if data_password.is_empty() {
        bail!("password is empty");
    }
    if socket::DATA_SOCKET
        .set(MessageSocket::new_client(DATA_CENTER_SOCKET, data_password))
        .is_err()
    {
        panic!("failed to set DATA_SOCKET");
    }

    let backend_password = read_password_from_tty(Some("backend password: "))?;
    if backend_password.is_empty() {
        bail!("password is empty");
    }

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREAD_NUM)
        .thread_name("acfunlivedata backend worker")
        .enable_all()
        .max_blocking_threads(MAX_BLOCKING_THREAD)
        .build()?
        .block_on(async {
            let mut config: config::LiveConfig = CommonConfig::new_or_load_config(
                backend_password.clone(),
                crate::config::CONFIG_FILE_PATH.as_path(),
            )
            .await
            .expect("failed to load config");
            if !config.contains_admin_token() {
                let token = config::generate_token();
                println!("admin token:\n{}", token);
                config.set_admin_token(token);
                config.save_config().await.expect("failed to save config");
            }
            if config::CONFIG.set(Arc::new(Mutex::new(config))).is_err() {
                panic!("failed to set CONFIG");
            };

            create_dir(&*download::TEMP_DIRECTORY)
                .await
                .expect("failed to create temp directory");

            tokio::select! {
                _ = socket::message(backend_password) => {}
                _ = server::graphql_server() => {}
            }
        });

    Ok(())
}
