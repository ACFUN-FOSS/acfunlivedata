use once_cell::sync::Lazy;
use std::path::PathBuf;

const DATABASE_DIR: &str = "database";
const LIVERS_DIR: &str = "livers";
pub const ACFUN_LIVE_DATABASE_NAME: &str = "acfunlive.db";
pub const GIFT_DATABASE_NAME: &str = "gift.db";

pub static DATABASE_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = crate::DIRECTORY_PATH.clone();
    path.push(DATABASE_DIR);
    path
});

pub static LIVERS_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DATABASE_DIRECTORY.clone();
    path.push(LIVERS_DIR);
    path
});

pub static ACFUN_LIVE_DATABASE: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DATABASE_DIRECTORY.clone();
    path.push(ACFUN_LIVE_DATABASE_NAME);
    path
});

pub static GIFT_DATABASE: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DATABASE_DIRECTORY.clone();
    path.push(GIFT_DATABASE_NAME);
    path
});

#[inline]
pub fn liver_db_file(liver_uid: i64) -> String {
    liver_uid.to_string() + ".db"
}

#[inline]
pub fn liver_db_path(liver_uid: i64) -> PathBuf {
    let mut path = LIVERS_DIRECTORY.clone();
    path.push(liver_db_file(liver_uid));
    path
}
