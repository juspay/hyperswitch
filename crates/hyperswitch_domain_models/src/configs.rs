use common_utils::errors::CustomResult;
use diesel_models::configs as storage;

#[async_trait::async_trait]
pub trait ConfigInterface {
    type Error;
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn find_config_by_key(&self, key: &str) -> CustomResult<storage::Config, Self::Error>;

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be created with the default value.
        default_config: Option<String>,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<storage::Config, Self::Error>;
}
