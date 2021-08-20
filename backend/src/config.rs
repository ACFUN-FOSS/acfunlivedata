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
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

const CONFIG_FILE: &str = "acfunlivedata_backend.json";
const TOKEN_LENGTH: usize = 20;

pub static CONFIG_FILE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DIRECTORY_PATH.clone();
    path.push(CONFIG_FILE);
    path
});

pub static CONFIG: OnceCell<Arc<Mutex<LiveConfig>>> = OnceCell::new();

pub type LiveConfig = CommonConfig<Config, &'static Path>;

type Users = AHashMap<String, User>;

#[inline]
pub fn generate_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect()
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum User {
    Admin,
    Liver(i64),
}

impl User {
    #[inline]
    pub fn is_admin(&self) -> bool {
        self == &User::Admin
    }

    #[inline]
    pub fn is_liver(&self, liver_uid: i64) -> bool {
        self == &User::Liver(liver_uid)
    }
}

#[derive(Clone, Debug, SimpleObject)]
pub struct TokenInfo {
    pub exist: bool,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    users: Users,
}

impl Config {
    #[inline]
    pub fn contains_uid(&self, liver_uid: i64) -> bool {
        if liver_uid > 0 {
            self.users.values().any(|i| i.is_liver(liver_uid))
        } else {
            false
        }
    }

    #[inline]
    pub fn contains_token(&self, token: &str) -> bool {
        self.users.contains_key(token)
    }

    #[inline]
    pub fn contains_admin_token(&self) -> bool {
        self.users.values().any(|i| i.is_admin())
    }

    #[inline]
    pub fn get(&self, token: &str) -> Option<User> {
        self.users.get(token).copied()
    }

    #[inline]
    pub fn set_admin_token(&mut self, token: String) {
        let _ = self.users.insert(token, User::Admin);
    }

    #[inline]
    pub fn set_liver_token(&mut self, token: String, liver_uid: i64) -> Result<()> {
        if liver_uid > 0 {
            let _ = self.users.insert(token, User::Liver(liver_uid));
            Ok(())
        } else {
            Err(anyhow::anyhow!("liver uid {} is less than 1", liver_uid))
        }
    }

    #[inline]
    pub fn is_admin_token(&self, token: &str) -> bool {
        if let Some(user) = self.users.get(token) {
            user.is_admin()
        } else {
            false
        }
    }

    pub async fn add_liver(&mut self, liver_uid: i64, tool: bool) -> Result<TokenInfo> {
        if liver_uid > 0 {
            log::info!("add liver {}", liver_uid);
            let token = generate_token();
            let mut exist = false;
            if self.contains_uid(liver_uid) {
                exist = true;
                log::warn!("already added liver {} before", liver_uid);
                self.users.retain(|_, i| !i.is_liver(liver_uid));
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
            self.set_liver_token(token.clone(), liver_uid)?;
            Ok(TokenInfo {
                exist,
                token: Some(token),
            })
        } else {
            bail!("liver uid {} is less than 1", liver_uid);
        }
    }

    pub async fn delete_liver(&mut self, liver_uid: i64, tool: bool) -> Result<TokenInfo> {
        if liver_uid > 0 {
            log::info!("delete liver {}", liver_uid);
            let mut exist = false;
            if self.contains_uid(liver_uid) {
                exist = true;
                self.users.retain(|_, i| !i.is_liver(liver_uid));
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
            Ok(TokenInfo { exist, token: None })
        } else {
            bail!("liver uid {} is less than 1", liver_uid);
        }
    }
}
