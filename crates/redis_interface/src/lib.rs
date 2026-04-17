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

pub mod commands;
pub mod constant;
pub mod errors;
pub mod types;

use std::sync::{atomic, Arc};

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use redis::AsyncCommands;

pub use self::types::*;

// ─── Cluster abstraction ────────────────────────────────────────────────────

/// An abstraction over standalone and cluster Redis connections.
/// Both variants implement `redis::aio::ConnectionLike`, so all
/// `AsyncCommands` work transparently on either.
#[derive(Clone)]
pub enum RedisConn {
    Standalone(redis::aio::ConnectionManager),
    Cluster(redis::cluster_async::ClusterConnection),
}

impl redis::aio::ConnectionLike for RedisConn {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a redis::Cmd) -> redis::RedisFuture<'a, Value> {
        match self {
            Self::Standalone(c) => c.req_packed_command(cmd),
            Self::Cluster(c) => c.req_packed_command(cmd),
        }
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a redis::Pipeline,
        offset: usize,
        count: usize,
    ) -> redis::RedisFuture<'a, Vec<Value>> {
        match self {
            Self::Standalone(c) => c.req_packed_commands(cmd, offset, count),
            Self::Cluster(c) => c.req_packed_commands(cmd, offset, count),
        }
    }

    fn get_db(&self) -> i64 {
        match self {
            Self::Standalone(c) => c.get_db(),
            Self::Cluster(c) => c.get_db(),
        }
    }
}

// ─── Subscriber client with auto-resubscribe ────────────────────────────────

/// A pub/sub subscriber that tracks its subscriptions and can resubscribe on reconnect.
pub struct SubscriberClient {
    client: redis::Client,
    pubsub: tokio::sync::Mutex<redis::aio::PubSub>,
    subscriptions: tokio::sync::RwLock<std::collections::HashSet<String>>,
    message_tx: tokio::sync::broadcast::Sender<PubSubMessage>,
    pub is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
}

/// Represents a message received from a pub/sub channel.
#[derive(Clone, Debug)]
pub struct PubSubMessage {
    pub channel: String,
    pub value: Value,
}

impl SubscriberClient {
    pub async fn new(client: redis::Client) -> CustomResult<Self, errors::RedisError> {
        let pubsub = client
            .get_async_pubsub()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;

        let (message_tx, _) = tokio::sync::broadcast::channel(256);

        Ok(Self {
            client,
            pubsub: tokio::sync::Mutex::new(pubsub),
            subscriptions: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            message_tx,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    /// Subscribe to a channel and track it for auto-resubscribe
    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        let mut pubsub = self.pubsub.lock().await;
        pubsub
            .subscribe(channel)
            .await
            .change_context(errors::RedisError::SubscribeError)?;

        let mut subs = self.subscriptions.write().await;
        subs.insert(channel.to_string());

        Ok(())
    }

    /// Unsubscribe from a channel and remove it from tracking
    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        let mut pubsub = self.pubsub.lock().await;
        pubsub
            .unsubscribe(channel)
            .await
            .change_context(errors::RedisError::SubscribeError)?;

        let mut subs = self.subscriptions.write().await;
        subs.remove(channel);

        Ok(())
    }

    /// Get a receiver for pub/sub messages
    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.message_tx.subscribe()
    }

    /// Spawn the message forwarding loop.
    /// Reads from the underlying PubSub and forwards messages to the broadcast channel.
    /// On disconnect, it attempts to reconnect and resubscribe.
    pub async fn manage_subscriptions(&self) {
        use futures::StreamExt;

        let tx = self.message_tx.clone();

        // We process messages in a loop; on error we attempt reconnection
        loop {
            // Hold the lock and process messages
            let result = {
                let mut pubsub = self.pubsub.lock().await;
                let msg = pubsub.on_message().next().await;
                drop(pubsub);
                msg
            };

            match result {
                Some(msg) => {
                    let channel = msg.get_channel_name().to_string();
                    let payload: Value = msg.get_payload().unwrap_or(Value::Nil);
                    let _ = tx.send(PubSubMessage {
                        channel,
                        value: payload,
                    });
                }
                None => {
                    // Stream ended — connection likely dropped. Try to reconnect.
                    tracing::warn!("PubSub connection dropped, attempting to reconnect");
                    let mut pubsub = self.pubsub.lock().await;
                    match self.reconnect_and_resubscribe(&mut pubsub).await {
                        Ok(()) => {
                            tracing::info!("PubSub reconnected and resubscribed successfully");
                        }
                        Err(e) => {
                            tracing::error!(?e, "Failed to reconnect PubSub");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Reconnect the underlying PubSub and resubscribe to all tracked channels
    async fn reconnect_and_resubscribe(
        &self,
        pubsub: &mut redis::aio::PubSub,
    ) -> CustomResult<(), errors::RedisError> {
        let new_pubsub = self
            .client
            .get_async_pubsub()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;
        *pubsub = new_pubsub;

        let subs = self.subscriptions.read().await;
        for channel in subs.iter() {
            pubsub
                .subscribe(channel.as_str())
                .await
                .change_context(errors::RedisError::SubscribeError)?;
        }
        Ok(())
    }
}

// ─── Publisher client ────────────────────────────────────────────────────────

/// A simple wrapper around a connection manager used for publishing.
pub struct RedisClient {
    inner: redis::aio::ConnectionManager,
}

impl RedisClient {
    pub async fn new(client: &redis::Client) -> CustomResult<Self, errors::RedisError> {
        let conn = redis::aio::ConnectionManager::new(client.clone())
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: conn })
    }

    /// Publish a message to a channel
    pub async fn publish(
        &self,
        channel: &str,
        message: RedisValue,
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.inner.clone();
        conn.publish::<_, _, usize>(channel, message)
            .await
            .change_context(errors::RedisError::PublishError)
    }
}

// ─── Connection pool ─────────────────────────────────────────────────────────

pub struct RedisConnectionPool {
    pub pool: RedisConn,
    pub key_prefix: String,
    pub config: Arc<RedisConfig>,
    pub subscriber: Arc<SubscriberClient>,
    pub publisher: Arc<RedisClient>,
    pub is_redis_available: Arc<atomic::AtomicBool>,
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    pub async fn new(conf: &RedisSettings) -> CustomResult<Self, errors::RedisError> {
        let redis_connection_url = format!("redis://{}:{}", conf.host, conf.port);

        let pool = if conf.cluster_enabled {
            // Build cluster connection
            let mut nodes = vec![redis_connection_url.clone()];
            for url in &conf.cluster_urls {
                // cluster_urls might be "host:port" or full URLs
                if url.starts_with("redis://") {
                    nodes.push(url.clone());
                } else {
                    nodes.push(format!("redis://{url}"));
                }
            }

            let cluster_client = redis::cluster::ClusterClient::new(nodes)
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to create Redis cluster client for {}:{}",
                        conf.host, conf.port
                    )
                })?;

            let cluster_conn = cluster_client
                .get_async_connection()
                .await
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to connect to Redis cluster at {}:{}",
                        conf.host, conf.port
                    )
                })?;

            RedisConn::Cluster(cluster_conn)
        } else {
            // Build standalone connection
            let client = redis::Client::open(redis_connection_url.as_str())
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to open Redis client for {}:{}",
                        conf.host, conf.port
                    )
                })?;

            let conn = redis::aio::ConnectionManager::new(client)
                .await
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!("Failed to connect to Redis at {}:{}", conf.host, conf.port)
                })?;

            RedisConn::Standalone(conn)
        };

        // Create a separate client for publisher and subscriber
        let base_client = redis::Client::open(redis_connection_url.as_str())
            .change_context(errors::RedisError::RedisConnectionError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to open Redis pub/sub client for {}:{}",
                    conf.host, conf.port
                )
            })?;

        let subscriber = SubscriberClient::new(base_client.clone()).await?;
        let publisher = RedisClient::new(&base_client).await?;

        let config = RedisConfig::from(conf);

        Ok(Self {
            pool,
            config: Arc::new(config),
            is_redis_available: Arc::new(atomic::AtomicBool::new(true)),
            subscriber: Arc::new(subscriber),
            publisher: Arc::new(publisher),
            key_prefix: String::default(),
        })
    }

    pub fn clone(&self, key_prefix: &str) -> Self {
        Self {
            pool: self.pool.clone(),
            key_prefix: key_prefix.to_string(),
            config: Arc::clone(&self.config),
            subscriber: Arc::clone(&self.subscriber),
            publisher: Arc::clone(&self.publisher),
            is_redis_available: Arc::clone(&self.is_redis_available),
        }
    }

    /// Monitor for connection errors.
    /// When Redis is unreachable for longer than `max_failure_threshold` seconds,
    /// signals via the oneshot sender and marks redis as unavailable.
    pub async fn on_error(&self, tx: tokio::sync::oneshot::Sender<()>) {
        let check_interval = self.config.unresponsive_check_interval.max(1);
        let max_unreachable_secs = self.config.max_failure_threshold;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval));
        let mut first_failure_at: Option<std::time::Instant> = None;

        loop {
            interval.tick().await;
            let mut conn = self.pool.clone();

            // Timeout the ping so we can check threshold frequently,
            // even when ConnectionManager is retrying internally
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(check_interval),
                conn.ping::<String>(),
            )
            .await;

            let ping_ok = matches!(result, Ok(Ok(_)));

            if ping_ok {
                if first_failure_at.is_some() {
                    tracing::info!("Redis connection restored");
                }
                first_failure_at = None;
                self.is_redis_available
                    .store(true, atomic::Ordering::SeqCst);
                continue;
            }

            let now = std::time::Instant::now();
            let first_failure = *first_failure_at.get_or_insert(now);
            let unreachable_secs = now.duration_since(first_failure).as_secs();

            if unreachable_secs >= u64::from(max_unreachable_secs) {
                tracing::error!(
                    "Redis has been unreachable for {}s (threshold: {}s), shutting down",
                    unreachable_secs,
                    max_unreachable_secs
                );
                let _ = tx.send(());
                self.is_redis_available
                    .store(false, atomic::Ordering::SeqCst);
                break;
            }

            tracing::warn!(
                "Redis unreachable for {}s (threshold: {}s), reconnecting",
                unreachable_secs,
                max_unreachable_secs
            );
        }
    }

    /// Monitor for unresponsive Redis servers by periodically sending PING
    /// and logging warnings if the response is slow.
    pub async fn on_unresponsive(&self) {
        let check_interval = self.config.unresponsive_check_interval.max(2);
        let max_timeout = self.config.unresponsive_timeout.max(5);

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval));

        loop {
            interval.tick().await;
            let mut conn = self.pool.clone();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(max_timeout),
                conn.ping::<String>(),
            )
            .await;

            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    tracing::warn!(?e, "Redis PING failed");
                }
                Err(_) => {
                    tracing::warn!("Redis server is unresponsive (PING timed out)");
                }
            }
        }
    }

    /// Get an atomic pipeline for transaction support
    pub fn get_pipeline(&self) -> redis::Pipeline {
        redis::pipe().atomic().clone()
    }
}

pub struct RedisConfig {
    pub(crate) default_ttl: u32,
    pub(crate) default_stream_read_count: u64,
    pub(crate) default_hash_ttl: u32,
    pub(crate) cluster_enabled: bool,
    pub(crate) unresponsive_timeout: u64,
    pub(crate) unresponsive_check_interval: u64,
    pub(crate) max_failure_threshold: u32,
}

impl From<&RedisSettings> for RedisConfig {
    fn from(config: &RedisSettings) -> Self {
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
    use super::*;

    #[test]
    fn test_redis_error() {
        let x = errors::RedisError::ConsumerGroupClaimFailed.to_string();

        assert_eq!(x, "Failed to set Redis stream message owner".to_string())
    }
}
