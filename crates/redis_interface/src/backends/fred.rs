//! Fred backend implementation for the redis_interface crate.
//!
//! This module contains the connection pool, subscriber, and publisher
//! implementations using the `fred` crate.

pub mod commands;
pub mod types;

use std::sync::{atomic, Arc};

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use fred::{clients::Transaction, interfaces::{ClientLike, EventInterface}, prelude::TransactionInterface};

use crate::types::RedisValue;

// ─── RedisClient (publisher) ────────────────────────────────────────────────

pub struct RedisClient {
    inner: fred::prelude::RedisClient,
}

impl RedisClient {
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
    ) -> CustomResult<Self, crate::errors::RedisError> {
        let client =
            fred::prelude::RedisClient::new(config, Some(perf), None, Some(reconnect_policy));
        client.connect();
        client
            .wait_for_connect()
            .await
            .change_context(crate::errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: client })
    }

    pub async fn publish(
        &self,
        channel: &str,
        message: RedisValue,
    ) -> CustomResult<usize, crate::errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner
            .publish(channel, message.into_inner())
            .await
            .change_context(crate::errors::RedisError::PublishError)
    }
}

// ─── SubscriberClient ────────────────────────────────────────────────────────

/// Represents a message received from a pub/sub channel.
#[derive(Clone, Debug)]
pub struct PubSubMessage {
    pub channel: String,
    pub value: RedisValue,
}

pub struct SubscriberClient {
    inner: fred::clients::SubscriberClient,
    pub is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
    broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
}

impl SubscriberClient {
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
        broadcast_capacity: usize,
    ) -> CustomResult<Self, crate::errors::RedisError> {
        let client =
            fred::clients::SubscriberClient::new(config, Some(perf), None, Some(reconnect_policy));
        client.connect();
        client
            .wait_for_connect()
            .await
            .change_context(crate::errors::RedisError::RedisConnectionError)?;
        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);

        // Auto-spawn the message forwarding task, just like the redis-rs backend.
        // This reads from fred's internal message broadcast and forwards to our
        // PubSubMessage broadcast channel, so callers can use `message_rx()`.
        let fred_rx = client.message_rx();
        let sender = broadcast_sender.clone();
        tokio::spawn(async move {
            Self::forward_messages(fred_rx, sender).await;
        });

        Ok(Self {
            inner: client,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
            broadcast_sender,
        })
    }

    /// Background task that reads from fred's internal message stream and
    /// forwards each message to our broadcast channel.
    async fn forward_messages(
        mut fred_rx: tokio::sync::broadcast::Receiver<fred::types::Message>,
        broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
    ) {
        loop {
            match fred_rx.recv().await {
                Ok(msg) => {
                    let channel = msg.channel.to_string();
                    let value = RedisValue::from_bytes(
                        msg.value
                            .as_bytes()
                            .map(|b: &[u8]| b.to_vec())
                            .unwrap_or_default(),
                    );
                    let _ = broadcast_sender.send(PubSubMessage { channel, value });
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("fred pub/sub receiver lagged, {n} messages dropped");
                }
            }
        }
    }

    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.broadcast_sender.subscribe()
    }

    pub fn is_subscriber_handler_spawned(&self) -> &Arc<atomic::AtomicBool> {
        &self.is_subscriber_handler_spawned
    }

    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), crate::errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner
            .subscribe(channel)
            .await
            .change_context(crate::errors::RedisError::SubscribeError)
    }

    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), crate::errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner
            .unsubscribe(channel)
            .await
            .change_context(crate::errors::RedisError::SubscribeError)
    }
}

// ─── Connection pool ─────────────────────────────────────────────────────────

pub struct RedisConnectionPool {
    pub pool: Arc<fred::prelude::RedisPool>,
    pub key_prefix: String,
    pub config: Arc<RedisConfig>,
    pub subscriber: Arc<SubscriberClient>,
    pub publisher: Arc<RedisClient>,
    pub is_redis_available: Arc<atomic::AtomicBool>,
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    pub async fn new(conf: &crate::types::RedisSettings) -> CustomResult<Self, crate::errors::RedisError> {
        let redis_connection_url = match conf.cluster_enabled {
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
            false => format!("redis://{}:{}", conf.host, conf.port,),
        };
        let mut config = fred::types::RedisConfig::from_url(&redis_connection_url)
            .change_context(crate::errors::RedisError::RedisConnectionError)?;

        let perf = fred::types::PerformanceConfig {
            auto_pipeline: conf.auto_pipeline,
            default_command_timeout: std::time::Duration::from_secs(conf.default_command_timeout),
            max_feed_count: conf.max_feed_count,
            backpressure: fred::types::BackpressureConfig {
                disable_auto_backpressure: conf.disable_auto_backpressure,
                max_in_flight_commands: conf.max_in_flight_commands,
                policy: fred::types::BackpressurePolicy::Drain,
            },
            broadcast_channel_capacity: conf.broadcast_channel_capacity,
        };

        let connection_config = fred::types::ConnectionConfig {
            unresponsive: fred::types::UnresponsiveConfig {
                max_timeout: Some(std::time::Duration::from_secs(conf.unresponsive_timeout)),
                interval: std::time::Duration::from_secs(conf.unresponsive_check_interval),
            },
            ..fred::types::ConnectionConfig::default()
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

        let subscriber = SubscriberClient::new(
            config.clone(),
            reconnect_policy.clone(),
            perf.clone(),
            conf.broadcast_channel_capacity,
        )
        .await?;

        let publisher =
            RedisClient::new(config.clone(), reconnect_policy.clone(), perf.clone()).await?;

        let pool = fred::prelude::RedisPool::new(
            config,
            Some(perf),
            Some(connection_config),
            Some(reconnect_policy),
            conf.pool_size,
        )
        .change_context(crate::errors::RedisError::RedisConnectionError)?;

        pool.connect();
        pool.wait_for_connect()
            .await
            .change_context(crate::errors::RedisError::RedisConnectionError)?;

        let config = RedisConfig::from(conf);

        Ok(Self {
            pool: Arc::new(pool),
            config: Arc::new(config),
            is_redis_available: Arc::new(atomic::AtomicBool::new(true)),
            subscriber: Arc::new(subscriber),
            publisher: Arc::new(publisher),
            key_prefix: String::default(),
        })
    }

    pub fn clone(&self, key_prefix: &str) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            key_prefix: key_prefix.to_string(),
            config: Arc::clone(&self.config),
            subscriber: Arc::clone(&self.subscriber),
            publisher: Arc::clone(&self.publisher),
            is_redis_available: Arc::clone(&self.is_redis_available),
        }
    }

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
                tracing::error!(?error, "Redis protocol or connection error");
                if self.pool.state() == fred::types::ClientState::Disconnected {
                    if tx.send(()).is_err() {
                        tracing::error!("The redis shutdown signal sender failed to signal");
                    }
                    self.is_redis_available
                        .store(false, atomic::Ordering::SeqCst);
                    break;
                }
            }
        }
    }

    pub async fn on_unresponsive(&self) {
        let _ = self.pool.clients().iter().map(|client| {
            client.on_unresponsive(|server| {
                tracing::warn!(redis_server =?server.host, "Redis server is unresponsive");
                Ok(())
            })
        });
    }

    pub fn get_transaction(&self) -> Transaction {
        self.pool.next().multi()
    }
}

// ─── RedisConfig ─────────────────────────────────────────────────────────────

pub struct RedisConfig {
    pub(crate) default_ttl: u32,
    pub(crate) default_stream_read_count: u64,
    pub(crate) default_hash_ttl: u32,
    pub(crate) cluster_enabled: bool,
    #[allow(dead_code)]
    pub(crate) unresponsive_timeout: u64,
    #[allow(dead_code)]
    pub(crate) unresponsive_check_interval: u64,
    // max_failure_threshold is present in RedisSettings but not used by fred-rs
    #[allow(dead_code)]
    pub(crate) max_failure_threshold: u32,
}

impl From<&crate::types::RedisSettings> for RedisConfig {
    fn from(config: &crate::types::RedisSettings) -> Self {
        Self {
            default_ttl: config.default_ttl,
            default_stream_read_count: config.stream_read_count,
            default_hash_ttl: config.default_hash_ttl,
            cluster_enabled: config.cluster_enabled,
            unresponsive_timeout: config.unresponsive_timeout,
            unresponsive_check_interval: config.unresponsive_check_interval,
            max_failure_threshold: config.max_failure_threshold,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::errors;

    #[test]
    fn test_redis_error() {
        let x = errors::RedisError::ConsumerGroupClaimFailed.to_string();
        assert_eq!(x, "Failed to set Redis stream message owner".to_string())
    }
}
