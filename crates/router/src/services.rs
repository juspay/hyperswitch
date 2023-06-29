pub mod api;
pub mod authentication;
pub mod encryption;
pub mod logger;

use std::sync::Arc;

use error_stack::{IntoReport, ResultExt};
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use storage_models::services as storage;

pub use self::{api::*, encryption::*, storage::*};
use crate::{
    cache::{CacheKind, ACCOUNTS_CACHE, CONFIG_CACHE},
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

pub trait RedisConnInterface {
    fn get_redis_conn(&self) -> Arc<redis_interface::RedisConnectionPool>;
}

impl RedisConnInterface for Store {
    fn get_redis_conn(&self) -> Arc<redis_interface::RedisConnectionPool> {
        self.redis_conn.clone()
    }
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
