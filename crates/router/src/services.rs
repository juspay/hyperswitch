pub mod api;
pub mod authentication;
pub mod encryption;
pub mod logger;

use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms::{self, decrypt::KmsDecrypt};
#[cfg(not(feature = "kms"))]
use masking::PeekInterface;
use masking::Secret;
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use storage_impl::{KVRouterStore, RouterStore};
use tokio::sync::oneshot;

pub use self::{api::*, encryption::*};
use crate::{
    cache::{CacheKind, ACCOUNTS_CACHE, CONFIG_CACHE},
    configs::settings,
    consts,
    core::errors,
};

#[async_trait::async_trait]
pub trait PubSubInterface {
    async fn subscribe(&self, channel: &str) -> errors::CustomResult<(), redis_errors::RedisError>;

    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> errors::CustomResult<usize, redis_errors::RedisError>;

    async fn on_message(&self) -> errors::CustomResult<(), redis_errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for redis_interface::RedisConnectionPool {
    #[inline]
    async fn subscribe(&self, channel: &str) -> errors::CustomResult<(), redis_errors::RedisError> {
        // Spawns a task that will automatically re-subscribe to any channels or channel patterns used by the client.
        self.subscriber.manage_subscriptions();

        self.subscriber
            .subscribe(channel)
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> errors::CustomResult<usize, redis_errors::RedisError> {
        self.publisher
            .publish(channel, RedisValue::from(key).into_inner())
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
    async fn on_message(&self) -> errors::CustomResult<(), redis_errors::RedisError> {
        logger::debug!("Started on message");
        let mut rx = self.subscriber.on_message();
        while let Ok(message) = rx.recv().await {
            logger::debug!("Invalidating {message:?}");
            let key: CacheKind<'_> = match RedisValue::new(message.value)
                .try_into()
                .change_context(redis_errors::RedisError::OnMessageError)
            {
                Ok(value) => value,
                Err(err) => {
                    logger::error!(value_conversion_err=?err);
                    continue;
                }
            };

            let key = match key {
                CacheKind::Config(key) => {
                    CONFIG_CACHE.invalidate(key.as_ref()).await;
                    key
                }
                CacheKind::Accounts(key) => {
                    ACCOUNTS_CACHE.invalidate(key.as_ref()).await;
                    key
                }
                CacheKind::All(key) => {
                    CONFIG_CACHE.invalidate(key.as_ref()).await;
                    ACCOUNTS_CACHE.invalidate(key.as_ref()).await;
                    key
                }
            };

            self.delete_key(key.as_ref())
                .await
                .map_err(|err| logger::error!("Error while deleting redis key: {err:?}"))
                .ok();

            logger::debug!("Done invalidating {key}");
        }
        Ok(())
    }
}

#[cfg(not(feature = "olap"))]
type StoreType = storage_impl::database::store::Store;
#[cfg(feature = "olap")]
type StoreType = storage_impl::database::store::ReplicaStore;

#[cfg(not(feature = "kv_store"))]
pub type Store = RouterStore<StoreType>;
#[cfg(feature = "kv_store")]
pub type Store = KVRouterStore<StoreType>;

pub async fn get_store(
    config: &settings::Settings,
    shut_down_signal: oneshot::Sender<()>,
    test_transaction: bool,
) -> Store {
    #[cfg(feature = "kms")]
    let kms_client = kms::get_kms_client(&config.kms).await;

    #[cfg(feature = "kms")]
    let master_config = config
        .master_database
        .clone()
        .decrypt_inner(kms_client)
        .await;
    #[cfg(not(feature = "kms"))]
    let master_config = config.master_database.clone().into();

    #[cfg(all(feature = "olap", feature = "kms"))]
    let replica_config = config
        .replica_database
        .clone()
        .decrypt_inner(kms_client)
        .await;

    #[cfg(all(feature = "olap", not(feature = "kms")))]
    let replica_config = config.replica_database.clone().into();

    let master_enc_key = get_master_enc_key(
        config,
        #[cfg(feature = "kms")]
        kms_client,
    )
    .await;
    #[cfg(not(feature = "olap"))]
    let conf = master_config;
    #[cfg(feature = "olap")]
    let conf = (master_config, replica_config);

    let store: RouterStore<StoreType> = if test_transaction {
        RouterStore::test_store(conf, &config.redis, master_enc_key).await
    } else {
        RouterStore::from_config(
            conf,
            &config.redis,
            master_enc_key,
            shut_down_signal,
            consts::PUB_SUB_CHANNEL,
        )
        .await
    };

    #[cfg(feature = "kv_store")]
    let store = KVRouterStore::from_store(
        store,
        config.drainer.stream_name.clone(),
        config.drainer.num_partitions,
    );

    store
}

#[allow(clippy::expect_used)]
async fn get_master_enc_key(
    conf: &crate::configs::settings::Settings,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
) -> Secret<Vec<u8>> {
    #[cfg(feature = "kms")]
    let master_enc_key = hex::decode(
        conf.secrets
            .master_enc_key
            .clone()
            .decrypt_inner(kms_client)
            .await
            .expect("Failed to decrypt master enc key"),
    )
    .expect("Failed to decode from hex");

    #[cfg(not(feature = "kms"))]
    let master_enc_key =
        hex::decode(conf.secrets.master_enc_key.peek()).expect("Failed to decode from hex");

    Secret::new(master_enc_key)
}

#[inline]
pub fn generate_aes256_key() -> errors::CustomResult<[u8; 32], common_utils::errors::CryptoError> {
    use ring::rand::SecureRandom;

    let rng = ring::rand::SystemRandom::new();
    let mut key: [u8; 256 / 8] = [0_u8; 256 / 8];
    rng.fill(&mut key)
        .into_report()
        .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
    Ok(key)
}
