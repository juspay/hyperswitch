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
use redis::{AsyncCommands, IntoConnectionInfo};

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
pub enum SubscriberClient {
    /// Standalone pub/sub using `redis::aio::PubSub`
    Standalone {
        redis_client: redis::Client,
        pubsub_connection: tokio::sync::Mutex<redis::aio::PubSub>,
        subscriptions: tokio::sync::RwLock<std::collections::HashSet<String>>,
        broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
        is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
    },
    /// Cluster pub/sub using RESP3 push messages via an mpsc channel
    Cluster {
        cluster_connection: tokio::sync::Mutex<redis::cluster_async::ClusterConnection>,
        push_receiver: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<redis::PushInfo>>,
        subscriptions: tokio::sync::RwLock<std::collections::HashSet<String>>,
        broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
        is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
    },
}

/// Represents a message received from a pub/sub channel.
#[derive(Clone, Debug)]
pub struct PubSubMessage {
    pub channel: String,
    pub value: Value,
}

impl SubscriberClient {
    /// Create a new standalone subscriber
    pub async fn new(redis_client: redis::Client, broadcast_capacity: usize) -> CustomResult<Self, errors::RedisError> {
        let pubsub = redis_client
            .get_async_pubsub()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;

        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);

        Ok(Self::Standalone {
            redis_client,
            pubsub_connection: tokio::sync::Mutex::new(pubsub),
            subscriptions: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            broadcast_sender,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    /// Create a new cluster subscriber using RESP3 push messages
    pub async fn new_cluster(
        nodes: Vec<String>,
        broadcast_capacity: usize,
    ) -> CustomResult<Self, errors::RedisError> {
        let (push_sender, push_receiver_channel) = tokio::sync::mpsc::unbounded_channel();

        let cluster_client = redis::cluster::ClusterClient::builder(nodes)
            .use_protocol(redis::ProtocolVersion::RESP3)
            .push_sender(push_sender)
            .build()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let cluster_conn = cluster_client
            .get_async_connection()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;

        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);

        Ok(Self::Cluster {
            cluster_connection: tokio::sync::Mutex::new(cluster_conn),
            push_receiver: tokio::sync::Mutex::new(push_receiver_channel),
            subscriptions: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            broadcast_sender,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    /// Subscribe to a channel and track it for auto-resubscribe
    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        match self {
            Self::Standalone { pubsub_connection, .. } => {
                let mut pubsub = pubsub_connection.lock().await;
                pubsub
                    .subscribe(channel)
                    .await
                    .change_context(errors::RedisError::SubscribeError)?;
            }
            Self::Cluster { cluster_connection, .. } => {
                let mut connection = cluster_connection.lock().await;
                connection.subscribe(channel)
                    .await
                    .change_context(errors::RedisError::SubscribeError)?;
            }
        }

        let mut subs = self.subscriptions().write().await;
        subs.insert(channel.to_string());

        Ok(())
    }

    /// Unsubscribe from a channel and remove it from tracking
    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        match self {
            Self::Standalone { pubsub_connection, .. } => {
                let mut pubsub = pubsub_connection.lock().await;
                pubsub
                    .unsubscribe(channel)
                    .await
                    .change_context(errors::RedisError::SubscribeError)?;
            }
            Self::Cluster { cluster_connection, .. } => {
                let mut connection = cluster_connection.lock().await;
                connection.unsubscribe(channel)
                    .await
                    .change_context(errors::RedisError::SubscribeError)?;
            }
        }

        let mut subs = self.subscriptions().write().await;
        subs.remove(channel);

        Ok(())
    }

    /// Get a receiver for pub/sub messages
    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.broadcast_sender().subscribe()
    }

    /// Spawn the message forwarding loop.
    /// Reads from the underlying PubSub (standalone) or push channel (cluster)
    /// and forwards messages to the broadcast channel.
    pub async fn manage_subscriptions(&self) {
        match self {
            Self::Standalone { pubsub_connection, redis_client, subscriptions, broadcast_sender, is_subscriber_handler_spawned, .. } => {
                Self::manage_standalone(pubsub_connection, redis_client, subscriptions, broadcast_sender, is_subscriber_handler_spawned).await
            }
            Self::Cluster { push_receiver, broadcast_sender, is_subscriber_handler_spawned, .. } => {
                Self::manage_cluster(push_receiver, broadcast_sender, is_subscriber_handler_spawned).await
            }
        }
    }

    // ── Standalone message loop ──────────────────────────────────────────────

    async fn manage_standalone(
        pubsub_connection: &tokio::sync::Mutex<redis::aio::PubSub>,
        redis_client: &redis::Client,
        subscriptions: &tokio::sync::RwLock<std::collections::HashSet<String>>,
        broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
        _is_subscriber_handler_spawned: &Arc<atomic::AtomicBool>,
    ) {
        use futures::StreamExt;

        let sender = broadcast_sender.clone();
        let mut retry_delay = constant::PUBSUB_INITIAL_RETRY_DELAY;

        loop {
            let result = {
                let mut pubsub = pubsub_connection.lock().await;
                let msg = pubsub.on_message().next().await;
                drop(pubsub);
                msg
            };

            match result {
                Some(msg) => {
                    retry_delay = constant::PUBSUB_INITIAL_RETRY_DELAY;
                    let channel = msg.get_channel_name().to_string();
                    let payload: Value = msg.get_payload().unwrap_or(Value::Nil);
                    let _ = sender.send(PubSubMessage {
                        channel,
                        value: payload,
                    });
                }
                None => {
                    tracing::warn!("PubSub connection dropped, attempting to reconnect");
                    let mut pubsub = pubsub_connection.lock().await;
                    match Self::reconnect_standalone(redis_client, &mut pubsub, subscriptions).await {
                        Ok(()) => {
                            retry_delay = constant::PUBSUB_INITIAL_RETRY_DELAY;
                            tracing::info!("PubSub reconnected and resubscribed successfully");
                        }
                        Err(e) => {
                            tracing::error!(?e, "Failed to reconnect PubSub, retrying in {:?}", retry_delay);
                            drop(pubsub);
                            tokio::time::sleep(retry_delay).await;
                            retry_delay = (retry_delay * constant::PUBSUB_RETRY_BACKOFF_FACTOR)
                                .min(constant::PUBSUB_MAX_RETRY_DELAY);
                        }
                    }
                }
            }
        }
        // If this loop ever exits, reset the flag so a new handler can be spawned
        // (currently this loop runs forever due to retry logic above)
        // is_subscriber_handler_spawned.store(false, atomic::Ordering::SeqCst);
    }

    async fn reconnect_standalone(
        redis_client: &redis::Client,
        pubsub: &mut redis::aio::PubSub,
        subscriptions: &tokio::sync::RwLock<std::collections::HashSet<String>>,
    ) -> CustomResult<(), errors::RedisError> {
        let new_pubsub = redis_client
            .get_async_pubsub()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;
        *pubsub = new_pubsub;

        let subs = subscriptions.read().await;
        for channel in subs.iter() {
            pubsub
                .subscribe(channel.as_str())
                .await
                .change_context(errors::RedisError::SubscribeError)?;
        }
        Ok(())
    }

    // ── Cluster message loop ─────────────────────────────────────────────────

    async fn manage_cluster(
        push_receiver: &tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<redis::PushInfo>>,
        broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
        _is_subscriber_handler_spawned: &Arc<atomic::AtomicBool>,
    ) {

        let sender = broadcast_sender.clone();
        let mut receiver = push_receiver.lock().await;

        while let Some(push_info) = receiver.recv().await {
            let channel = push_info
                .data
                .first()
                .map(|v| match v {
                    Value::BulkString(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                    Value::SimpleString(s) => s.clone(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            let payload = push_info.data.into_iter().nth(1).unwrap_or(Value::Nil);

            let _ = sender.send(PubSubMessage {
                channel,
                value: payload,
            });
        }
    }

    // ── Shared accessors ─────────────────────────────────────────────────────

    fn subscriptions(&self) -> &tokio::sync::RwLock<std::collections::HashSet<String>> {
        match self {
            Self::Standalone { subscriptions, .. } => subscriptions,
            Self::Cluster { subscriptions, .. } => subscriptions,
        }
    }

    fn broadcast_sender(&self) -> &tokio::sync::broadcast::Sender<PubSubMessage> {
        match self {
            Self::Standalone { broadcast_sender, .. } => broadcast_sender,
            Self::Cluster { broadcast_sender, .. } => broadcast_sender,
        }
    }

    pub fn is_subscriber_handler_spawned(&self) -> &Arc<atomic::AtomicBool> {
        match self {
            Self::Standalone { is_subscriber_handler_spawned, .. } => is_subscriber_handler_spawned,
            Self::Cluster { is_subscriber_handler_spawned, .. } => is_subscriber_handler_spawned,
        }
    }
}

// ─── Publisher client ────────────────────────────────────────────────────────

/// A simple wrapper around a connection used for publishing.
pub struct RedisClient {
    inner: RedisConn,
}

impl RedisClient {
    pub async fn new(client: &redis::Client) -> CustomResult<Self, errors::RedisError> {
        let conn = redis::aio::ConnectionManager::new(client.clone())
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: RedisConn::Standalone(conn) })
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

            let cluster_conn = redis::cluster::ClusterClient::new(nodes)
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to create Redis cluster client for {}:{}",
                        conf.host, conf.port
                    )
                })?
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
            // Build standalone connection.
            //
            // Design note: `ConnectionManager` uses a single multiplexed TCP connection
            // with automatic reconnection (exponential backoff). This differs from the
            // old `fred::RedisPool` which maintained `pool_size` physical connections.
            // A single multiplexed connection is sufficient for most workloads because
            // Redis itself is single-threaded. The `pool_size` config field has no
            // equivalent — if a connection pool is needed, consider `bb8-redis`.
            let mut connection_info = redis_connection_url
                .as_str()
                .into_connection_info()
                .change_context(errors::RedisError::RedisConnectionError)?;

            if !conf.use_legacy_version {
                let redis_settings = connection_info
                    .redis_settings()
                    .clone()
                    .set_protocol(redis::ProtocolVersion::RESP3);
                connection_info = connection_info.set_redis_settings(redis_settings);
            }

            let client = redis::Client::open(connection_info)
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to open Redis client for {}:{}",
                        conf.host, conf.port
                    )
                })?;

            // Build ConnectionManagerConfig from RedisSettings
            let reconnection_retries =
                usize::try_from(conf.reconnect_max_attempts).unwrap_or_default();
            let reconnection_min_delay = std::time::Duration::from_millis(
                u64::from(conf.reconnect_delay),
            );

            let mut connection_manager_config = redis::aio::ConnectionManagerConfig::new()
                .set_number_of_retries(reconnection_retries)
                .set_min_delay(reconnection_min_delay);

            if conf.default_command_timeout > 0 {
                connection_manager_config = connection_manager_config.set_response_timeout(Some(
                    std::time::Duration::from_secs(conf.default_command_timeout),
                ));
            }

            if conf.max_in_flight_commands > 0 {
                let pipeline_buffer_size =
                    usize::try_from(conf.max_in_flight_commands).unwrap_or_default();
                connection_manager_config =
                    connection_manager_config.set_pipeline_buffer_size(pipeline_buffer_size);
            }

            let conn = redis::aio::ConnectionManager::new_with_config(client, connection_manager_config)
                .await
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!("Failed to connect to Redis at {}:{}", conf.host, conf.port)
                })?;

            RedisConn::Standalone(conn)
        };

        // Create a separate client for publisher and subscriber.
        // In cluster mode, use the cluster client so pub/sub goes through the cluster.
        // In standalone mode, use the same redis://host:port.
        let (subscriber, publisher) = if conf.cluster_enabled {
            let mut nodes = vec![redis_connection_url.clone()];
            for url in &conf.cluster_urls {
                if url.starts_with("redis://") {
                    nodes.push(url.clone());
                } else {
                    nodes.push(format!("redis://{url}"));
                }
            }

            let cluster_conn = redis::cluster::ClusterClient::builder(nodes.clone())
                .build()
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to create Redis cluster pub/sub client for {}:{}",
                        conf.host, conf.port
                    )
                })?
                .get_async_connection()
                .await
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to connect Redis cluster pub/sub at {}:{}",
                        conf.host, conf.port
                    )
                })?;

            let publisher = RedisClient {
                inner: RedisConn::Cluster(cluster_conn),
            };
            let subscriber = SubscriberClient::new_cluster(nodes, conf.broadcast_channel_capacity).await?;

            (Arc::new(subscriber), Arc::new(publisher))
        } else {
            let mut base_connection_info = redis_connection_url
                .as_str()
                .into_connection_info()
                .change_context(errors::RedisError::RedisConnectionError)?;

            if !conf.use_legacy_version {
                let redis_settings = base_connection_info
                    .redis_settings()
                    .clone()
                    .set_protocol(redis::ProtocolVersion::RESP3);
                base_connection_info = base_connection_info.set_redis_settings(redis_settings);
            }

            let base_client = redis::Client::open(base_connection_info)
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to open Redis pub/sub client for {}:{}",
                        conf.host, conf.port
                    )
                })?;

            let subscriber = Arc::new(SubscriberClient::new(base_client.clone(), conf.broadcast_channel_capacity).await?);
            let publisher = Arc::new(RedisClient::new(&base_client).await?);

            (subscriber, publisher)
        };

        let config = RedisConfig::from(conf);

        Ok(Self {
            pool,
            config: Arc::new(config),
            is_redis_available: Arc::new(atomic::AtomicBool::new(true)),
            subscriber,
            publisher,
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
        let mut first_failure_at: Option<std::time::Instant> = None;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;
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

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;
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
