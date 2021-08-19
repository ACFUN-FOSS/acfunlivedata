mod config;
mod graphql;
mod model;
mod pool;
mod socket;
mod sql;
mod sqlite;

use acfunlivedata_common::{
    config::Config as CommonConfig,
    message::{MessageSocket, DATA_CENTER_SOCKET},
};
use anyhow::{bail, Result};
use axum::{prelude::*, AddExtensionLayer};
use rpassword::read_password_from_tty;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tower::{limit::concurrency::ConcurrencyLimitLayer, timeout::TimeoutLayer};
use tower_http::compression::CompressionLayer;

const WORKER_THREAD_NUM: usize = 10;
const MAX_BLOCKING_THREAD: usize = 2048;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const CONCURRENCY_LIMIT: usize = 50;

fn main() -> Result<()> {
    env_logger::builder()
        .filter(Some("acfunliveapi"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata_common"), log::LevelFilter::Trace)
        .filter(Some("acfunlivedata_backend"), log::LevelFilter::Trace)
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

    let schema = graphql::schema();
    //println!("{}", schema.sdl());

    let app = route(
        "/",
        get(graphql::graphql_playground).post(graphql::graphql_handler),
    )
    .layer(AddExtensionLayer::new(schema))
    .layer(CompressionLayer::new().gzip(true).no_deflate().no_br())
    .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
    .layer(ConcurrencyLimitLayer::new(CONCURRENCY_LIMIT));

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
            if !config.contains_super_token() {
                let token = config::generate_token();
                println!("super auth token:\n{}", token);
                config.set_super_token(token);
                config.save_config().await.expect("failed to save config");
            }
            if config::CONFIG.set(Arc::new(Mutex::new(config))).is_err() {
                panic!("failed to set CONFIG");
            };

            let serve = async {
                axum::Server::bind(&"0.0.0.0:3000".parse().expect("failed to parse ip address"))
                    .serve(app.into_make_service())
                    .await
                    .expect("failed to serve")
            };

            tokio::select! {
                _ = socket::message(backend_password) => {}
                _ = serve => {}
            }
        });

    Ok(())
}
