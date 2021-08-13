use crate::live::{LiveMessage, LIVE_TX};
use acfunlivedata_common::message::{DataCenterMessage, MessageSocket, DATA_CENTER_SOCKET};
use anyhow::bail;

pub async fn message(password: String) {
    let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
    let server: MessageSocket<_, DataCenterMessage> =
        MessageSocket::new_server(DATA_CENTER_SOCKET, password);

    loop {
        if let Err(e) = server
            .listen(|m| async move {
                if let Err(e) = live_tx.send(LiveMessage::Command(m)) {
                    bail!("failed to send LiveMessage: {}", e);
                }
                Ok(())
            })
            .await
        {
            log::error!("failed to listen socket: {}", e);
        }
    }
}
