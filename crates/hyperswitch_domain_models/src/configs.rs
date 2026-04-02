use common_utils::errors::CustomResult;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ConfigNew {
    pub key: String,
    pub config: String,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub key: String,
    pub config: String,
}

#[derive(Debug)]
pub enum ConfigUpdate {
    Update { config: Option<String> },
}

impl From<ConfigNew> for Config {
    fn from(config_new: ConfigNew) -> Self {
        Self {
            key: config_new.key,
            config: config_new.config,
        }
    }
}

#[async_trait::async_trait]
pub trait ConfigInterface {
    type Error;
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<Config, Self::Error>;

    async fn find_config_by_key(&self, key: &str) -> CustomResult<Config, Self::Error>;

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        default_config: Option<String>,
    ) -> CustomResult<Config, Self::Error>;

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<Config, Self::Error>;

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, Self::Error>;

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, Self::Error>;

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<Config, Self::Error>;
}
