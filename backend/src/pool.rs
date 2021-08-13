// code from https://github.com/LawnGnome/bb8-rusqlite/blob/main/src/lib.rs

use async_trait::async_trait;
use bb8::ManageConnection;
use rusqlite::{Connection as SqliteConn, OpenFlags};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub type Connection<'a> = bb8::PooledConnection<'a, RusqliteConnectionManager>;

/// A `bb8::ManageConnection` implementation for `rusqlite::Connection`
/// instances.
#[derive(Clone, Debug)]
pub struct RusqliteConnectionManager(Arc<ConnectionOptions>);

#[derive(Clone, Debug)]
struct ConnectionOptions {
    flags: rusqlite::OpenFlags,
    path: PathBuf,
}

/// Error wraps errors from both rusqlite and tokio.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A rusqlite error.
    #[error("rusqlite error")]
    Rusqlite(#[from] rusqlite::Error),

    /// A tokio join handle error.
    #[error("tokio join error")]
    TokioJoin(#[from] tokio::task::JoinError),
}

impl RusqliteConnectionManager {
    /// Analogous to `rusqlite::Connection::open_with_flags()`.
    pub fn new<P>(path: P, flags: OpenFlags) -> Self
    where
        P: AsRef<Path>,
    {
        Self(Arc::new(ConnectionOptions {
            flags,
            path: path.as_ref().into(),
        }))
    }
}

#[async_trait]
impl ManageConnection for RusqliteConnectionManager {
    type Connection = SqliteConn;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let options = self.0.clone();

        // Technically, we don't need to use spawn_blocking() here, but doing so
        // means we won't inadvertantly block this task for any length of time,
        // since rusqlite is inherently synchronous.
        Ok(tokio::task::spawn_blocking(move || {
            rusqlite::Connection::open_with_flags(&options.path, options.flags)
        })
        .await??)
    }

    async fn is_valid(
        &self,
        conn: &mut bb8::PooledConnection<'_, Self>,
    ) -> Result<(), Self::Error> {
        // Matching bb8-postgres, we'll try to run a trivial query here. Using
        // block_in_place() gives better behaviour if the SQLite call blocks for
        // some reason, but means that we depend on the tokio multi-threaded
        // runtime being active. (We can't use spawn_blocking() here because
        // Connection isn't Sync.)
        tokio::task::block_in_place(|| conn.execute("SELECT 1", []))?;
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        // There's no real concept of a "broken" connection in SQLite: if the
        // handle is still open, then we're good. (And we know the handle is
        // still open, because Connection::close() consumes the Connection, in
        // which case we're definitely not here.)
        false
    }
}
