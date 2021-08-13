pub mod config;
pub mod data;
pub mod database;
pub mod message;
pub mod socket;

use anyhow::Result;
use once_cell::sync::Lazy;
use std::{
    env::current_exe,
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::fs;

pub static DIRECTORY_PATH: Lazy<PathBuf> = Lazy::new(|| {
    current_exe()
        .expect("failed to get the path of the current running executable")
        .parent()
        .expect("the path is root")
        .to_path_buf()
});

#[inline]
pub async fn create_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if !file_exist(&path).await {
        fs::create_dir(&path).await?;
    }
    fs::set_permissions(&path, Permissions::from_mode(0o700)).await?;

    Ok(())
}

#[inline]
pub async fn file_exist<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).await.is_ok()
}
