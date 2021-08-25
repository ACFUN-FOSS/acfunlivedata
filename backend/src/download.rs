use crate::config::User;
use acfunlivedata_common::{
    client::build_client,
    database::{liver_db_file, liver_db_path},
    file_exist, DIRECTORY_PATH,
};
use anyhow::{bail, Error, Result};
use axum::{
    body::Body,
    http::{
        header::{HeaderMap, HeaderValue, CONTENT_DISPOSITION, CONTENT_LENGTH},
        Request,
    },
};
use cached::proc_macro::cached;
use once_cell::sync::Lazy;
use std::{
    convert::TryInto,
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::fs;
use tower::Service;

const TEMP_DIR: &str = "temp";

pub static TEMP_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DIRECTORY_PATH.clone();
    path.push(TEMP_DIR);
    path
});

#[cached(size = 10, time = 10, result = true)]
#[inline]
async fn is_live(liver_uid: i64) -> Result<bool> {
    let api_client = build_client().await?;
    let info = api_client.get_user_live_info(liver_uid).await?;
    if let Some(data) = info.live_data {
        Ok(!data.live_id.is_empty())
    } else {
        Ok(false)
    }
}

#[inline]
fn temp_db_path(liver_uid: i64) -> PathBuf {
    let mut path = TEMP_DIRECTORY.clone();
    path.push(liver_db_file(liver_uid));
    path
}

#[inline]
fn db_filename(liver_uid: i64) -> String {
    let date = chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%d-%H-%M-%S")
        .to_string();
    format!("{}-{}.db", liver_uid, date)
}

pub type DownloadFuture =
    Pin<Box<dyn Future<Output = std::result::Result<(PathBuf, HeaderMap), Error>> + Send>>;

#[derive(Clone, Copy, Debug)]
pub struct Download;

impl Service<Request<Body>> for Download {
    type Response = (PathBuf, HeaderMap);

    type Error = Error;

    type Future = DownloadFuture;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(async move {
            let liver_uid = match req.extensions().get::<User>() {
                Some(User::Liver(liver_uid)) => *liver_uid,
                Some(User::Admin) => bail!("this is an admin token"),
                None => panic!("no User in Request extensions"),
            };
            log::info!("[{}] start preparing downloading database", liver_uid);
            if is_live(liver_uid).await? {
                bail!("liver {} is living", liver_uid);
            }
            let db_path = liver_db_path(liver_uid);
            if !file_exist(&db_path).await {
                bail!("database file of liver {} doesn't exist", liver_uid);
            }

            let temp_path = temp_db_path(liver_uid);
            let _ = fs::copy(&db_path, &temp_path).await?;
            let metadata = fs::metadata(&temp_path).await?;
            let mut headers = HeaderMap::<HeaderValue>::with_capacity(2);
            let _ = headers.insert(CONTENT_LENGTH, metadata.len().into());
            let _ = headers.insert(
                CONTENT_DISPOSITION,
                format!(r#"attachment; filename="{}""#, db_filename(liver_uid)).try_into()?,
            );

            Ok((temp_path, headers))
        })
    }
}
