use crate::socket::send_data_message;
use acfunlivedata_common::{
    config::Config as CommonConfig,
    message::{send_tool_message, DataCenterMessage, ToolMessage},
    DIRECTORY_PATH,
};
use ahash::AHashMap;
use anyhow::{bail, Result};
use async_graphql::SimpleObject;
use once_cell::sync::{Lazy, OnceCell};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

const CONFIG_FILE: &str = "acfunlivedata_backend.json";
const TOKEN_LENGTH: usize = 20;
pub const SUPER_TOKEN_UID: i64 = 0;

pub static CONFIG_FILE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DIRECTORY_PATH.clone();
    path.push(CONFIG_FILE);
    path
});

pub static CONFIG: OnceCell<Arc<Mutex<LiveConfig>>> = OnceCell::new();

pub type Livers = AHashMap<String, i64>;
pub type LiveConfig = CommonConfig<Config, PathBuf>;

#[inline]
pub fn generate_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect()
}

#[derive(Clone, Debug, SimpleObject)]
pub struct Token {
    pub exist: bool,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    livers: Livers,
}

impl Config {
    #[inline]
    pub fn contains_uid(&self, liver_uid: i64) -> bool {
        self.livers.values().any(|i| *i == liver_uid)
    }

    #[inline]
    pub fn contains_token(&self, token: &str) -> bool {
        self.livers.contains_key(token)
    }

    #[inline]
    pub fn contains_super_token(&self) -> bool {
        self.contains_uid(SUPER_TOKEN_UID)
    }

    #[inline]
    pub fn get(&self, token: &str) -> Option<&i64> {
        self.livers.get(token)
    }

    #[inline]
    pub fn set_super_token(&mut self, token: String) {
        let _ = self.livers.insert(token, SUPER_TOKEN_UID);
    }

    #[inline]
    pub fn is_super_token(&self, token: &str) -> bool {
        if let Some(liver_uid) = self.livers.get(token) {
            *liver_uid == SUPER_TOKEN_UID
        } else {
            false
        }
    }

    pub async fn add_liver(&mut self, liver_uid: i64, tool: bool) -> Result<Token> {
        if liver_uid > 0 {
            log::info!("add liver {}", liver_uid);
            let token = generate_token();
            let mut exist = false;
            if self.contains_uid(liver_uid) {
                exist = true;
                log::warn!("already added liver {} before", liver_uid);
                self.livers.retain(|_, i| *i != liver_uid);
                if tool {
                    send_tool_message(&ToolMessage::BackendAddLiver(
                        liver_uid,
                        true,
                        token.clone(),
                    ))
                    .await;
                }
            } else if tool {
                send_tool_message(&ToolMessage::BackendAddLiver(
                    liver_uid,
                    false,
                    token.clone(),
                ))
                .await;
            }
            if !tool {
                send_data_message(&DataCenterMessage::AddLiver(liver_uid, false)).await;
            }
            let _ = self.livers.insert(token.clone(), liver_uid);
            Ok(Token {
                exist,
                token: Some(token),
            })
        } else {
            bail!("liver uid {} is less than 1", liver_uid);
        }
    }

    pub async fn delete_liver(&mut self, liver_uid: i64, tool: bool) -> Result<Token> {
        if liver_uid > 0 {
            log::info!("delete liver {}", liver_uid);
            let mut exist = false;
            if self.contains_uid(liver_uid) {
                exist = true;
                self.livers.retain(|_, i| *i != liver_uid);
                if tool {
                    send_tool_message(&ToolMessage::BackendDeleteLiver(liver_uid, true)).await;
                }
            } else {
                log::warn!("liver {} wasn't in config", liver_uid);
                if tool {
                    send_tool_message(&ToolMessage::BackendDeleteLiver(liver_uid, false)).await;
                }
            }
            if !tool {
                send_data_message(&DataCenterMessage::DeleteLiver(liver_uid, false)).await;
            }
            Ok(Token { exist, token: None })
        } else {
            bail!("liver uid {} is less than 1", liver_uid);
        }
    }
}
