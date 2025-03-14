pub mod api;
pub mod authentication;
pub mod authorization;
pub mod connector_integration_interface;
pub mod conversion_impls;
#[cfg(feature = "email")]
pub mod email;
pub mod encryption;
#[cfg(feature = "olap")]
pub mod jwt;
pub mod kafka;
pub mod logger;
pub mod pm_auth;

pub mod card_testing_guard;
#[cfg(feature = "olap")]
pub mod openidconnect;

use std::sync::Arc;

use error_stack::ResultExt;
use hyperswitch_domain_models::errors::StorageResult;
pub use hyperswitch_interfaces::connector_integration_v2::{
    BoxedConnectorIntegrationV2, ConnectorIntegrationAnyV2, ConnectorIntegrationV2,
};
use masking::{ExposeInterface, StrongSecret};
#[cfg(feature = "kv_store")]
use storage_impl::kv_router_store::KVRouterStore;
use storage_impl::{config::TenantConfig, redis::RedisStore, RouterStore};
use tokio::sync::oneshot;

pub use self::{api::*, encryption::*};
use crate::{configs::Settings, core::errors};

#[cfg(not(feature = "olap"))]
pub type StoreType = storage_impl::database::store::Store;
#[cfg(feature = "olap")]
pub type StoreType = storage_impl::database::store::ReplicaStore;

#[cfg(not(feature = "kv_store"))]
pub type Store = RouterStore<StoreType>;
#[cfg(feature = "kv_store")]
pub type Store = KVRouterStore<StoreType>;

/// # Panics
///
/// Will panic if hex decode of master key fails
#[allow(clippy::expect_used)]
pub async fn get_store(
    config: &Settings,
    tenant: &dyn TenantConfig,
    cache_store: Arc<RedisStore>,
    test_transaction: bool,
) -> StorageResult<Store> {
    let master_config = config.master_database.clone().into_inner();

    #[cfg(feature = "olap")]
    let replica_config = config.replica_database.clone().into_inner();

    #[allow(clippy::expect_used)]
    let master_enc_key = hex::decode(config.secrets.get_inner().master_enc_key.clone().expose())
        .map(StrongSecret::new)
        .expect("Failed to decode master key from hex");

    #[cfg(not(feature = "olap"))]
    let conf = master_config.into();
    #[cfg(feature = "olap")]
    // this would get abstracted, for all cases
    #[allow(clippy::useless_conversion)]
    let conf = (master_config.into(), replica_config.into());

    let store: RouterStore<StoreType> = if test_transaction {
        RouterStore::test_store(conf, tenant, &config.redis, master_enc_key).await?
    } else {
        RouterStore::from_config(
            conf,
            tenant,
            master_enc_key,
            cache_store,
            storage_impl::redis::cache::IMC_INVALIDATION_CHANNEL,
        )
        .await?
    };

    #[cfg(feature = "kv_store")]
    let store = KVRouterStore::from_store(
        store,
        config.drainer.stream_name.clone(),
        config.drainer.num_partitions,
        config.kv_config.ttl,
        config.kv_config.soft_kill,
    );

    Ok(store)
}

#[allow(clippy::expect_used)]
pub async fn get_cache_store(
    config: &Settings,
    shut_down_signal: oneshot::Sender<()>,
    _test_transaction: bool,
) -> StorageResult<Arc<RedisStore>> {
    RouterStore::<StoreType>::cache_store(&config.redis, shut_down_signal).await
}

#[inline]
pub fn generate_aes256_key() -> errors::CustomResult<[u8; 32], common_utils::errors::CryptoError> {
    use ring::rand::SecureRandom;

    let rng = ring::rand::SystemRandom::new();
    let mut key: [u8; 256 / 8] = [0_u8; 256 / 8];
    rng.fill(&mut key)
        .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
    Ok(key)
}
