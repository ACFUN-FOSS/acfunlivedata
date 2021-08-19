use acfunlivedata_common::{
    config::Config as CommonConfig,
    message::{send_tool_message, ToolMessage},
    DIRECTORY_PATH,
};
use ahash::AHashSet;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "acfunlivedata.json";

pub static CONFIG_FILE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = DIRECTORY_PATH.clone();
    path.push(CONFIG_FILE);
    path
});

pub type Livers = AHashSet<i64>;
pub type LiveConfig = CommonConfig<Config, &'static Path>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    livers: Livers,
}

impl Config {
    #[inline]
    pub fn contains(&self, liver_uid: i64) -> bool {
        self.livers.contains(&liver_uid)
    }

    #[inline]
    pub async fn add_liver(&mut self, liver_uid: i64, tool: bool) {
        if liver_uid > 0 {
            log::info!("add liver {}", liver_uid);
            if self.livers.insert(liver_uid) {
                if tool {
                    send_tool_message(&ToolMessage::DataCenterAddLiver(liver_uid, false)).await;
                }
            } else {
                log::warn!("already added liver {} before", liver_uid);
                if tool {
                    send_tool_message(&ToolMessage::DataCenterAddLiver(liver_uid, true)).await;
                }
            }
        } else {
            log::warn!("liver uid {} is less than 1", liver_uid);
        }
    }

    #[inline]
    pub async fn delete_liver(&mut self, liver_uid: i64, tool: bool) {
        if liver_uid > 0 {
            log::info!("delete liver {}", liver_uid);
            if self.livers.remove(&liver_uid) {
                if tool {
                    send_tool_message(&ToolMessage::DataCenterDeleteLiver(liver_uid, true)).await;
                }
            } else {
                log::warn!("liver {} wasn't in config", liver_uid);
                if tool {
                    send_tool_message(&ToolMessage::DataCenterDeleteLiver(liver_uid, false)).await;
                }
            }
        } else {
            log::warn!("liver uid {} is less than 1", liver_uid);
        }
    }
}
