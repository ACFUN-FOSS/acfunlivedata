use crate::pool::RusqliteConnectionManager;
use acfunlivedata_common::file_exist;
use anyhow::{bail, Result};
use bb8::Pool;
use cached::proc_macro::cached;
use once_cell::sync::Lazy;
use rusqlite::OpenFlags;
use std::{path::PathBuf, time::Duration};

const POOL_SIZE: u32 = 30;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

static OPEN_FLAGS: Lazy<OpenFlags> =
    Lazy::new(|| OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX);

#[cached(time = 3600, result = true)]
#[inline]
pub async fn connect(path: PathBuf) -> Result<Pool<RusqliteConnectionManager>> {
    if !file_exist(&path).await {
        bail!("database file doesn't exist");
    }

    Ok(Pool::builder()
        .max_size(POOL_SIZE)
        .max_lifetime(None)
        .idle_timeout(None)
        .connection_timeout(CONNECT_TIMEOUT)
        .build(RusqliteConnectionManager::new(path, *OPEN_FLAGS))
        .await?)
}
