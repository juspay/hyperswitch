pub mod api;
pub mod authentication;
pub mod authorization;
pub mod encryption;
#[cfg(feature = "olap")]
pub mod jwt;
pub mod kafka;
pub mod logger;
pub mod pm_auth;
#[cfg(feature = "recon")]
pub mod recon;

#[cfg(feature = "email")]
pub mod email;

use data_models::errors::StorageResult;
use error_stack::ResultExt;
use masking::{ExposeInterface, StrongSecret};
#[cfg(feature = "kv_store")]
use storage_impl::KVRouterStore;
use storage_impl::RouterStore;
use tokio::sync::oneshot;

pub use self::{api::*, encryption::*};
use crate::{configs::Settings, consts, core::errors};

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
    shut_down_signal: oneshot::Sender<()>,
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
        RouterStore::test_store(conf, &config.redis, master_enc_key).await?
    } else {
        RouterStore::from_config(
            conf,
            &config.redis,
            master_enc_key,
            shut_down_signal,
            consts::PUB_SUB_CHANNEL,
        )
        .await?
    };

    #[cfg(feature = "kv_store")]
    let store = KVRouterStore::from_store(
        store,
        config.drainer.stream_name.clone(),
        config.drainer.num_partitions,
        config.kv_config.ttl,
    );

    Ok(store)
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
