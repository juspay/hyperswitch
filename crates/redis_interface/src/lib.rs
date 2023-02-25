//! Intermediate module for encapsulate all the redis related functionality
//!
//! Provides structs to represent redis connection and all functions that redis provides and
//! are used in the `router` crate. Abstractions for creating a new connection while also facilitating
//! redis connection pool and configuration based types.
//!
//!  # Examples
//! ```
//! pub mod types;
//! use self::types;
//!
//! #[tokio::main]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let redis_conn = RedisConnectionPool::new(types::RedisSettings::default()).await;
//!     // ... redis_conn ready to use
//! }
//! ```
#![forbid(unsafe_code)]

pub mod commands;
pub mod errors;
pub mod types;

use std::sync::{atomic, Arc};

use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use fred::interfaces::{ClientLike, PubsubInterface};
use futures::StreamExt;
use router_env::logger;

pub use self::{commands::*, types::*};

pub struct RedisConnectionPool {
    pub pool: fred::pool::RedisPool,
    config: RedisConfig,
    join_handles: Vec<fred::types::ConnectHandle>,
    subscriber: RedisClient,
    publisher: RedisClient,
    pub is_redis_available: Arc<atomic::AtomicBool>,
}

pub struct RedisClient {
    inner: fred::prelude::RedisClient,
}

impl std::ops::Deref for RedisClient {
    type Target = fred::prelude::RedisClient;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RedisClient {
    pub async fn new(
        config: fred::types::RedisConfig,
        policy: fred::types::ReconnectPolicy,
    ) -> CustomResult<Self, errors::RedisError> {
        let client = fred::prelude::RedisClient::new(config);
        client.connect(Some(policy));
        client
            .wait_for_connect()
            .await
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: client })
    }
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    pub async fn new(conf: &RedisSettings) -> CustomResult<Self, errors::RedisError> {
        let redis_connection_url = match conf.cluster_enabled {
            // Fred relies on this format for specifying cluster where the host port is ignored & only query parameters are used for node addresses
            // redis-cluster://username:password@host:port?node=bar.com:30002&node=baz.com:30003
            true => format!(
                "redis-cluster://{}:{}?{}",
                conf.host,
                conf.port,
                conf.cluster_urls
                    .iter()
                    .flat_map(|url| vec!["&", url])
                    .skip(1)
                    .collect::<String>()
            ),
            false => format!(
                "redis://{}:{}", //URI Schema
                conf.host, conf.port,
            ),
        };
        let mut config = fred::types::RedisConfig::from_url(&redis_connection_url)
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;

        if !conf.use_legacy_version {
            config.version = fred::types::RespVersion::RESP3;
        }
        config.tracing = true;
        config.blocking = fred::types::Blocking::Error;
        let policy = fred::types::ReconnectPolicy::new_constant(
            conf.reconnect_max_attempts,
            conf.reconnect_delay,
        );

        let subscriber = RedisClient::new(config.clone(), policy.clone()).await?;

        let publisher = RedisClient::new(config.clone(), policy.clone()).await?;

        let pool = fred::pool::RedisPool::new(config, conf.pool_size)
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let join_handles = pool.connect(Some(policy));
        pool.wait_for_connect()
            .await
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let config = RedisConfig::from(conf);

        Ok(Self {
            pool,
            config,
            join_handles,
            is_redis_available: Arc::new(atomic::AtomicBool::new(true)),
            subscriber,
            publisher,
        })
    }

    pub async fn close_connections(&mut self) {
        self.pool.quit_pool().await;
        for handle in self.join_handles.drain(..) {
            match handle.await {
                Ok(Ok(_)) => (),
                Ok(Err(error)) => logger::error!(%error),
                Err(error) => logger::error!(%error),
            };
        }
    }
    pub async fn on_error(&self) {
        self.pool
            .on_error()
            .for_each(|err| {
                logger::error!("{err:?}");
                if self.pool.state() == fred::types::ClientState::Disconnected {
                    self.is_redis_available
                        .store(false, atomic::Ordering::SeqCst);
                }
                futures::future::ready(())
            })
            .await;
    }
}

#[async_trait::async_trait]
pub trait PubSubInterface {
    async fn subscribe(&self, channel: &str) -> CustomResult<usize, errors::RedisError>;
    async fn publish(&self, channel: &str, key: &str) -> CustomResult<usize, errors::RedisError>;
    async fn on_message(&self) -> CustomResult<(), errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for RedisConnectionPool {
    #[inline]
    async fn subscribe(&self, channel: &str) -> CustomResult<usize, errors::RedisError> {
        self.subscriber
            .subscribe(channel)
            .await
            .into_report()
            .change_context(errors::RedisError::SubscribeError)
    }
    #[inline]
    async fn publish(&self, channel: &str, key: &str) -> CustomResult<usize, errors::RedisError> {
        self.publisher
            .publish(channel, key)
            .await
            .into_report()
            .change_context(errors::RedisError::SubscribeError)
    }
    #[inline]
    async fn on_message(&self) -> CustomResult<(), errors::RedisError> {
        let mut message = self.subscriber.on_message();
        while let Some((_, key)) = message.next().await {
            let key = key
                .as_string()
                .ok_or::<errors::RedisError>(errors::RedisError::DeleteFailed)?;
            self.delete_key(&key).await?;
        }
        Ok(())
    }
}

struct RedisConfig {
    default_ttl: u32,
    default_stream_read_count: u64,
    default_hash_ttl: u32,
}

impl From<&RedisSettings> for RedisConfig {
    fn from(config: &RedisSettings) -> Self {
        Self {
            default_ttl: config.default_ttl,
            default_stream_read_count: config.stream_read_count,
            default_hash_ttl: config.default_hash_ttl,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_redis_error() {
        let x = errors::RedisError::ConsumerGroupClaimFailed.to_string();

        assert_eq!(x, "Failed to set Redis stream message owner".to_string())
    }
}
