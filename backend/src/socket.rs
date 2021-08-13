use crate::config::CONFIG;
use acfunlivedata_common::message::{
    BackendMessage, DataCenterMessage, MessageSocket, BACKEND_SOCKET,
};
use once_cell::sync::OnceCell;

pub static DATA_SOCKET: OnceCell<MessageSocket<&str, DataCenterMessage>> = OnceCell::new();

pub async fn send_data_message(message: &DataCenterMessage) {
    if let Err(e) = DATA_SOCKET
        .get()
        .expect("failed to get DATA_SOCKET")
        .send(message)
        .await
    {
        log::error!("failed to send {:?} to data center: {}", message, e);
    }
}

pub async fn message(password: String) {
    let server: MessageSocket<_, BackendMessage> =
        MessageSocket::new_server(BACKEND_SOCKET, password);

    loop {
        if let Err(e) = server
            .listen(|m| async move {
                let mut config = CONFIG.get().expect("failed to get CONFIG").lock().await;
                match m {
                    BackendMessage::AddLiver(liver_uid) => {
                        if let Err(e) = config.add_liver(liver_uid, true).await {
                            log::warn!("add liver error: {}", e);
                        }
                    }
                    BackendMessage::DeleteLiver(liver_uid) => {
                        if let Err(e) = config.delete_liver(liver_uid, true).await {
                            log::warn!("delete liver error: {}", e);
                        }
                    }
                }
                config.save_config().await?;
                Ok(())
            })
            .await
        {
            log::error!("failed to listen socket: {}", e);
        }
    }
}
