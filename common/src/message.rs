use crate::socket::Socket;
use anyhow::Result;
use asynchronous_codec::{BytesCodec, Framed};
use encon::Password;
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, io, marker::PhantomData, ops::Deref, time::Duration};
use tokio::time;

pub const DATA_CENTER_SOCKET: &str = "/tmp/acfunlivedata.sock";
pub const BACKEND_SOCKET: &str = "/tmp/acfunlivedata_backend.sock";
pub const TOOL_SOCKET: &str = "/tmp/acfunlivedata_tool.sock";
pub const TOOL_PASSWORD: &str = "acfunlivedata-tool";

const TIMEOUT: Duration = Duration::from_secs(5);

pub static TOOL_SOCKET_CLIENT: Lazy<MessageSocket<&str, ToolMessage>> =
    Lazy::new(|| MessageSocket::new_client(TOOL_SOCKET, TOOL_PASSWORD));

#[inline]
pub async fn send_tool_message(message: &ToolMessage) {
    if let Err(e) = TOOL_SOCKET_CLIENT.send(message).await {
        log::error!("failed to send {:?} to tool: {}", message, e);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum DataCenterMessage {
    AddLiver(i64, bool),
    DeleteLiver(i64, bool),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum BackendMessage {
    AddLiver(i64),
    DeleteLiver(i64),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ToolMessage {
    DataCenterAddLiver(i64, bool),
    DataCenterDeleteLiver(i64, bool),
    BackendAddLiver(i64, bool, String),
    BackendDeleteLiver(i64, bool),
}

#[derive(Clone)]
pub struct MessageSocket<P, M> {
    password: Password,
    socket: Socket<P>,
    message: PhantomData<M>,
}

impl<P, M> MessageSocket<P, M> {
    #[inline]
    pub fn new_server(path: P, password: impl Into<String>) -> Self {
        Self {
            password: Password::new(password),
            socket: Socket::new(path, true),
            message: PhantomData,
        }
    }

    #[inline]
    pub fn new_client(path: P, password: impl Into<String>) -> Self {
        Self {
            password: Password::new(password),
            socket: Socket::new(path, false),
            message: PhantomData,
        }
    }

    #[inline]
    pub fn is_server(&self) -> bool {
        self.socket.is_server()
    }
}

impl<M> MessageSocket<&'static str, M>
where
    M: DeserializeOwned,
{
    #[inline]
    pub async fn listen<F, Fut>(&self, mut f: F) -> Result<()>
    where
        F: FnMut(M) -> Fut + Copy,
        Fut: Future<Output = Result<()>>,
    {
        self.socket
            .listen(|conn| async move {
                let mut framed = Framed::new(conn, BytesCodec);
                let bytes = match framed.next().await.transpose()? {
                    Some(b) => b,
                    None => return Ok(()),
                };
                let msg: M = match bincode::deserialize(
                    &self
                        .password
                        .decrypt(bytes.deref())
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                ) {
                    Ok(msg) => msg,
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, e));
                    }
                };
                f(msg)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(())
            })
            .await
    }
}

impl<M> MessageSocket<&'static str, M>
where
    M: Serialize,
{
    #[inline]
    pub async fn send(&self, message: &M) -> Result<()> {
        let msg = self.password.encrypt(bincode::serialize(message)?)?;
        time::timeout(TIMEOUT, self.socket.write(msg)).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_message_socket() -> Result<()> {
        let server: MessageSocket<_, DataCenterMessage> =
            MessageSocket::new_server(DATA_CENTER_SOCKET, "abcd");
        let client: MessageSocket<_, DataCenterMessage> =
            MessageSocket::new_client(DATA_CENTER_SOCKET, "abcd");
        let _ = tokio::spawn(async move {
            server
                .listen(|m| async move {
                    assert_eq!(m, DataCenterMessage::AddLiver(100, false));
                    Ok(())
                })
                .await
                .unwrap();
        });
        sleep(Duration::from_secs(2)).await;
        client
            .send(&DataCenterMessage::AddLiver(100, false))
            .await?;
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }
}
