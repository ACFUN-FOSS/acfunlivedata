use crate::file_exist;
use anyhow::{bail, Result};
use encon::{Encryptable, Map, Password};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Cow,
    fs::Permissions,
    ops::{Deref, DerefMut},
    os::unix::fs::PermissionsExt,
    path::Path,
};
use tokio::fs;

#[derive(Clone)]
pub struct Config<C, P> {
    config: C,
    password: Password,
    path: P,
}

impl<C, P> Config<C, P> {
    #[inline]
    pub fn set_config(&mut self, config: C) {
        self.config = config;
    }
}

impl<C, P> Deref for Config<C, P> {
    type Target = C;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl<C, P> DerefMut for Config<C, P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl<C: Default, P> Config<C, P> {
    #[inline]
    pub fn new(password: impl Into<String>, path: P) -> Self {
        Self {
            config: C::default(),
            password: Password::new(password),
            path,
        }
    }
}

impl<C, P: AsRef<Path>> Config<C, P> {
    #[inline]
    pub async fn file_exist(&self) -> bool {
        file_exist(&self.path).await
    }
}

impl<C, P> Config<C, P>
where
    C: Serialize,
{
    pub fn encrypt(&self) -> Result<String> {
        let mut map = Map::new();
        let _ = map.insert(
            "config",
            Encryptable::Plain(serde_json::to_value(&self.config)?).with_intent_encrypted(),
        );
        if let Err(e) = map.apply_all_intents(&self.password) {
            bail!("failed to encrypt config: {}", e);
        }

        Ok(map.to_json_compact()?)
    }
}

impl<C, P> Config<C, P>
where
    C: Serialize,
    P: AsRef<Path>,
{
    #[inline]
    pub async fn save_config(&self) -> Result<()> {
        let encrypted = self.encrypt()?;
        fs::write(&self.path, encrypted).await?;
        fs::set_permissions(&self.path, Permissions::from_mode(0o600)).await?;

        Ok(())
    }
}

impl<C, P> Config<C, P>
where
    C: DeserializeOwned,
{
    pub fn decrypt<'a>(&mut self, encrypt_config: impl Into<Cow<'a, str>>) -> Result<C> {
        let mut map: Map = serde_json::from_str(&encrypt_config.into())?;
        if let Err(e) = map.decrypt_all_in_place(&self.password) {
            bail!("failed to decrypt config: {}", e);
        }
        if let Some(intend) = map.remove("config") {
            if let Encryptable::Plain(value) = intend.into_inner() {
                Ok(std::mem::replace(
                    &mut self.config,
                    serde_json::from_value(value)?,
                ))
            } else {
                bail!("config in WithIntend is not plain");
            }
        } else {
            bail!("failed to get `config` field in the config map");
        }
    }
}

impl<C, P> Config<C, P>
where
    C: DeserializeOwned,
    P: AsRef<Path>,
{
    #[inline]
    pub async fn load_config(&mut self) -> Result<C> {
        let config = fs::read(&self.path).await?;
        let config = String::from_utf8_lossy(&config);
        self.decrypt(config)
    }
}

impl<C, P> Config<C, P>
where
    C: Default + DeserializeOwned,
    P: AsRef<Path>,
{
    #[inline]
    pub async fn new_or_load_config(password: impl Into<String>, path: P) -> Result<Self> {
        let mut config = Self::new(password, path);
        if config.file_exist().await {
            let _ = config.load_config().await?;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_encrypt_decrypt() -> Result<()> {
        #[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
        struct TestConfig {
            test: String,
        }

        let test_config = TestConfig {
            test: "foo".to_string(),
        };
        let mut config: Config<TestConfig, _> = Config::new("bar", "");
        config.set_config(test_config);
        let encrypted = config.encrypt()?;
        let mut new_config: Config<TestConfig, _> = Config::new("bar", "");
        let old = new_config.decrypt(encrypted)?;
        assert_eq!(old, TestConfig::default());
        assert_eq!(*config, *new_config);

        Ok(())
    }
}
