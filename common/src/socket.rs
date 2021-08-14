use crate::file_exist;
use anyhow::{bail, Result};
use asynchronous_codec::{Framed, LengthCodec};
use futures::{SinkExt, TryStreamExt};
use interprocess::nonblocking::local_socket::{LocalSocketListener, LocalSocketStream};
use std::{future::Future, path::Path};
use tokio::fs;

#[derive(Clone, Debug)]
pub(crate) struct Socket<P> {
    path: P,
    is_server: bool,
}

impl<P> Socket<P> {
    #[inline]
    pub(crate) fn new(path: P, is_server: bool) -> Self {
        Self { path, is_server }
    }

    #[inline]
    pub(crate) fn is_server(&self) -> bool {
        self.is_server
    }
}

impl<P: AsRef<Path>> Socket<P> {
    #[inline]
    async fn socket_exist(&self) -> bool {
        file_exist(&self.path).await
    }

    #[inline]
    async fn remove_socket(&self) -> Result<()> {
        Ok(fs::remove_file(&self.path).await?)
    }
}

impl Socket<&'static str> {
    pub(crate) async fn listen<F, Fut>(&self, f: F) -> Result<()>
    where
        F: FnMut(LocalSocketStream) -> Fut,
        Fut: Future<Output = std::result::Result<(), std::io::Error>>,
    {
        if !self.is_server {
            bail!("not a server");
        }
        if self.socket_exist().await {
            self.remove_socket().await?
        }
        let socket = LocalSocketListener::bind(self.path).await?.incoming();
        socket.try_for_each_concurrent(None, f).await?;

        Ok(())
    }

    #[inline]
    pub(crate) async fn write(&self, message: Vec<u8>) -> Result<()> {
        if self.is_server {
            bail!("not a client");
        }
        let conn = LocalSocketStream::connect(self.path).await?;
        let mut framed = Framed::new(conn, LengthCodec);
        framed.send(message.into()).await?;
        framed.close().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::BACKEND_SOCKET;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_socket() -> Result<()> {
        let server = Socket::new(BACKEND_SOCKET, true);
        let client = Socket::new(BACKEND_SOCKET, false);
        let _ = tokio::spawn(async move {
            server
                .listen(|conn| async move {
                    let mut framed = Framed::new(conn, LengthCodec);
                    let msg = framed.next().await.transpose()?.unwrap();
                    assert_eq!(msg.as_ref(), b"hello, server");
                    Ok(())
                })
                .await
                .unwrap();
        });
        sleep(Duration::from_secs(2)).await;
        client.write(b"hello, server".to_vec()).await?;
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }
}
