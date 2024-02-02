//! Intermediate module for encapsulate all the redis related functionality
//!
//! Provides structs to represent redis connection and all functions that redis provides and
//! are used in the `router` crate. Abstractions for creating a new connection while also facilitating
//! redis connection pool and configuration based types.
//!
//!  # Examples
//! ```
//! use redis_interface::{types::RedisSettings, RedisConnectionPool};
//!
//! #[tokio::main]
//! async fn main() {
//!     let redis_conn = RedisConnectionPool::new(&RedisSettings::default()).await;
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
pub use fred::interfaces::PubsubInterface;
use fred::{interfaces::ClientLike, prelude::EventInterface};
use router_env::logger;

pub use self::types::*;

pub struct RedisConnectionPool {
    pub pool: fred::prelude::RedisPool,
    config: RedisConfig,
    pub subscriber: SubscriberClient,
    pub publisher: RedisClient,
    pub is_redis_available: Arc<atomic::AtomicBool>,
}

pub struct RedisClient {
    inner: fred::prelude::RedisClient,
}

impl std::ops::Deref for RedisClient {
    type Target = fred::prelude::RedisClient;
        /// This method returns a reference to the inner value of the smart pointer.
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RedisClient {
        /// Creates a new instance of the RedisConnectionManager with the provided configuration settings.
    /// 
    /// # Arguments
    /// - `config` - The Redis configuration settings.
    /// - `reconnect_policy` - The policy for reconnecting to the Redis server.
    /// - `perf` - The performance configuration settings.
    /// 
    /// # Returns
    /// A `CustomResult` containing the newly created `RedisConnectionManager` if successful, or a `RedisError` if an error occurs.
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
    ) -> CustomResult<Self, errors::RedisError> {
        let client =
            fred::prelude::RedisClient::new(config, Some(perf), None, Some(reconnect_policy));
        client.connect();
        client
            .wait_for_connect()
            .await
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: client })
    }
}

pub struct SubscriberClient {
    inner: fred::clients::SubscriberClient,
}

impl SubscriberClient {
        /// Creates a new instance of the SubscriberService with the provided Redis configuration, reconnect policy, and performance configuration. 
    /// This method initializes a new SubscriberClient with the given configuration and connects to the Redis server. It then waits for the client to connect and returns a CustomResult containing the initialized SubscriberService on success, or a RedisError on failure.
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
    ) -> CustomResult<Self, errors::RedisError> {
        let client =
            fred::clients::SubscriberClient::new(config, Some(perf), None, Some(reconnect_policy));
        client.connect();
        client
            .wait_for_connect()
            .await
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: client })
    }
}

impl std::ops::Deref for SubscriberClient {
    type Target = fred::clients::SubscriberClient;
    fn deref(&self) -> &Self::Target {
        &self.inner
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

        let perf = fred::types::PerformanceConfig {
            auto_pipeline: conf.auto_pipeline,
            default_command_timeout: std::time::Duration::from_secs(conf.default_command_timeout),
            max_feed_count: conf.max_feed_count,
            backpressure: fred::types::BackpressureConfig {
                disable_auto_backpressure: conf.disable_auto_backpressure,
                max_in_flight_commands: conf.max_in_flight_commands,
                policy: fred::types::BackpressurePolicy::Drain,
            },
        };

        if !conf.use_legacy_version {
            config.version = fred::types::RespVersion::RESP3;
        }
        config.tracing = fred::types::TracingConfig::new(true);
        config.blocking = fred::types::Blocking::Error;
        let reconnect_policy = fred::types::ReconnectPolicy::new_constant(
            conf.reconnect_max_attempts,
            conf.reconnect_delay,
        );

        let subscriber =
            SubscriberClient::new(config.clone(), reconnect_policy.clone(), perf.clone()).await?;

        let publisher =
            RedisClient::new(config.clone(), reconnect_policy.clone(), perf.clone()).await?;

        let pool = fred::prelude::RedisPool::new(
            config,
            Some(perf),
            None,
            Some(reconnect_policy),
            conf.pool_size,
        )
        .into_report()
        .change_context(errors::RedisError::RedisConnectionError)?;

        pool.connect();
        pool.wait_for_connect()
            .await
            .into_report()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let config = RedisConfig::from(conf);

        Ok(Self {
            pool,
            config,
            is_redis_available: Arc::new(atomic::AtomicBool::new(true)),
            subscriber,
            publisher,
        })
    }

        /// Asynchronously monitors and handles errors from the Redis clients in the pool. If a Redis protocol or connection error occurs, it logs the error and checks if the Redis pool is in a disconnected state. If it is disconnected, it sends a shutdown signal through the provided `tx` sender and updates the availability status of the Redis pool. This method utilizes the provided `tx` sender to communicate the shutdown signal if the pool is disconnected, and it uses the `logger` to log any encountered errors.
    pub async fn on_error(&self, tx: tokio::sync::oneshot::Sender<()>) {
        use futures::StreamExt;
        use tokio_stream::wrappers::BroadcastStream;

        let error_rxs: Vec<BroadcastStream<fred::error::RedisError>> = self
            .pool
            .clients()
            .iter()
            .map(|client| BroadcastStream::new(client.error_rx()))
            .collect();

        let mut error_rx = futures::stream::select_all(error_rxs);
        loop {
            if let Some(Ok(error)) = error_rx.next().await {
                logger::error!(?error, "Redis protocol or connection error");
                if self.pool.state() == fred::types::ClientState::Disconnected {
                    if tx.send(()).is_err() {
                        logger::error!("The redis shutdown signal sender failed to signal");
                    }
                    self.is_redis_available
                        .store(false, atomic::Ordering::SeqCst);
                    break;
                }
            }
        }
    }
}

struct RedisConfig {
    default_ttl: u32,
    default_stream_read_count: u64,
    default_hash_ttl: u32,
}

impl From<&RedisSettings> for RedisConfig {
        /// Constructs a new instance of Self using the provided RedisSettings configuration.
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
        /// This method tests the Redis error handling by creating a Redis error instance and checking if its string representation matches a predefined value.
    fn test_redis_error() {
        let x = errors::RedisError::ConsumerGroupClaimFailed.to_string();

        assert_eq!(x, "Failed to set Redis stream message owner".to_string())
    }
}
