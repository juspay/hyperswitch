use common_utils::errors::CustomResult;
use diesel_models::configs as storage;

/// Every method on this trait is expected to go through the caching layer:
/// reads are served from (and populate) the config cache, and writes redact the
/// cached entry across all instances. Direct database access is intentionally
/// not exposed here.
#[async_trait::async_trait]
pub trait ConfigInterface {
    type Error;
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn find_config_by_key_optional(
        &self,
        key: &str,
    ) -> CustomResult<Option<storage::Config>, Self::Error>;

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found this default value is used (and cached).
        default_config: String,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, Self::Error>;

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<storage::Config, Self::Error>;
}
