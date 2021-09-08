#![forbid(unsafe_code)]

use acfunlivedata_common::message::{
    BackendMessage, DataCenterMessage, MessageSocket, ToolMessage, BACKEND_SOCKET,
    DATA_CENTER_SOCKET, TOOL_PASSWORD, TOOL_SOCKET,
};
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use rpassword::read_password_from_tty;
use std::{sync::Arc, time::Duration};
use structopt::StructOpt;
use tokio::{sync::Mutex, time};

const SLEEP: Duration = Duration::from_secs(2);
const SERVER_RUN_TIME: Duration = Duration::from_secs(10);

static NUM: Lazy<Arc<Mutex<usize>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "acfunlivedata-tool", about = "A tool for acfunlivedata.")]
struct Opt {
    #[structopt(
        short,
        long,
        value_name("liver uid"),
        help("add livers"),
        required_unless("del")
    )]
    add: Vec<i64>,
    #[structopt(
        short,
        long,
        value_name("liver uid"),
        help("delete livers"),
        required_unless("add")
    )]
    del: Vec<i64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    let opt_num = opt.add.len() + opt.del.len();
    if opt.add.iter().chain(opt.del.iter()).any(|i| *i <= 0) {
        bail!("some liver uids in the option are less than 1");
    }

    let data_center_password = read_password_from_tty(Some("data center password: "))?;
    if data_center_password.is_empty() {
        bail!("data center password is empty");
    }
    let backend_password = read_password_from_tty(Some("backend password: "))?;
    if backend_password.is_empty() {
        bail!("backend password is empty");
    }

    let _ = tokio::spawn(async move {
        time::sleep(SLEEP).await;
        let data_client: MessageSocket<_, DataCenterMessage> =
            MessageSocket::new_client(DATA_CENTER_SOCKET, data_center_password);
        for liver_uid in &opt.add {
            data_client
                .send(&DataCenterMessage::AddLiver(*liver_uid, true))
                .await
                .expect("failed to send DataCenterMessage::AddLiver");
        }
        for liver_uid in &opt.del {
            data_client
                .send(&DataCenterMessage::DeleteLiver(*liver_uid, true))
                .await
                .expect("failed to send DataCenterMessage::DeleteLiver");
        }

        let backend_client: MessageSocket<_, BackendMessage> =
            MessageSocket::new_client(BACKEND_SOCKET, backend_password);
        for liver_uid in &opt.add {
            backend_client
                .send(&BackendMessage::AddLiver(*liver_uid))
                .await
                .expect("failed to send BackendMessage::AddLiver");
        }
        for liver_uid in &opt.del {
            backend_client
                .send(&BackendMessage::DeleteLiver(*liver_uid))
                .await
                .expect("failed to send BackendMessage::DeleteLiver");
        }
    });

    let server = async {
        let server: MessageSocket<_, ToolMessage> =
            MessageSocket::new_server(TOOL_SOCKET, TOOL_PASSWORD);
        server
            .listen(|m| async move {
                {
                    let mut num = NUM.lock().await;
                    *num += 1;
                }
                match m {
                    ToolMessage::DataCenterAddLiver(liver_uid, exist) => {
                        if exist {
                            println!("liver uid {} is already in acfunlivedata config", liver_uid);
                        } else {
                            println!(
                                "add liver uid {} in acfunlivedata config successfully",
                                liver_uid
                            );
                        }
                    }
                    ToolMessage::DataCenterDeleteLiver(liver_uid, exist) => {
                        if exist {
                            println!(
                                "delete liver uid {} in acfunlivedata config successfully",
                                liver_uid
                            );
                        } else {
                            println!("liver uid {} is not in acfunlivedata config", liver_uid);
                        }
                    }
                    ToolMessage::BackendAddLiver(liver_uid, exist, token) => {
                        if exist {
                            println!(
                                "liver uid {} is already in acfunlivedata-backend config",
                                liver_uid
                            );
                            println!(
                                "liver uid {} already has token\nnow generate new token:\n{}",
                                liver_uid, token
                            );
                        } else {
                            println!(
                                "add liver uid {} in acfunlivedata-backend config successfully",
                                liver_uid
                            );
                            println!("generate liver uid {} token:\n{}", liver_uid, token);
                        }
                    }
                    ToolMessage::BackendDeleteLiver(liver_uid, exist) => {
                        if exist {
                            println!(
                                "delete liver uid {} in acfunlivedata-backend config successfully",
                                liver_uid
                            );
                        } else {
                            println!(
                                "liver uid {} is not in acfunlivedata-backend config",
                                liver_uid
                            );
                        }
                    }
                };
                Ok(())
            })
            .await
    };
    tokio::select! {
        Err(e) = server => {
            println!("server listen error: {}", e);
        }
        _ = time::sleep(SERVER_RUN_TIME) => {}
    }

    let num = NUM.lock().await;
    if *num != 2 * opt_num {
        println!(
            "failed to complete all operations, maybe passwords were wrong or the socket was timeout"
        );
    }

    Ok(())
}
