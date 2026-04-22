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

/// A pub/sub subscriber that uses RESP3 push messages for both standalone and cluster.
///
/// Both modes use the same push-message mechanism: the underlying connection
/// sends push messages (subscribe confirmations, messages, etc.) through an
/// mpsc channel, which this struct forwards to a broadcast channel for consumers.
pub struct SubscriberClient {
    /// Sender for subscribe/unsubscribe commands to the background task
    command_sender: tokio::sync::mpsc::Sender<SubscriberCommand>,
    /// Broadcast sender — consumers call `message_rx()` to get a receiver
    broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
    /// Whether the background message-handling task has been spawned
    is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
}

/// Commands sent from the public API to the background subscriber task
enum SubscriberCommand {
    Subscribe {
        channel: String,
        done: tokio::sync::oneshot::Sender<()>,
    },
    Unsubscribe {
        channel: String,
        done: tokio::sync::oneshot::Sender<()>,
    },
}

/// Represents a message received from a pub/sub channel.
#[derive(Clone, Debug)]
pub struct PubSubMessage {
    pub channel: String,
    pub value: Value,
}

/// The backend owned by the background task — variant-specific connection logic
enum SubscriberBackend {
    Standalone {
        connection: redis::aio::ConnectionManager,
    },
    Cluster {
        connection: redis::cluster_async::ClusterConnection,
    },
}

impl SubscriberClient {
    /// Create a new standalone subscriber using RESP3 push messages
    pub async fn new(
        redis_client: redis::Client,
        broadcast_capacity: usize,
        connection_manager_config: redis::aio::ConnectionManagerConfig,
    ) -> CustomResult<Self, errors::RedisError> {
        let (push_sender, _) =
            tokio::sync::broadcast::channel::<redis::PushInfo>(broadcast_capacity);

        let config = connection_manager_config
            .set_push_sender(push_sender.clone())
            .set_automatic_resubscription();

        let connection = redis::aio::ConnectionManager::new_with_config(
            redis_client.clone(),
            config,
        )
        .await
        .change_context(errors::RedisError::RedisConnectionError)
        .attach_printable_lazy(|| {
            format!(
                "Failed to create subscriber connection for {}",
                redis_client.get_connection_info().addr()
            )
        })?;

        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(64);

        let push_receiver = push_sender.subscribe();

        let backend = SubscriberBackend::Standalone {
            connection,
        };

        tokio::spawn(Self::run(backend, push_receiver, broadcast_sender.clone(), command_receiver));

        Ok(Self {
            command_sender,
            broadcast_sender,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    /// Create a new cluster subscriber using RESP3 push messages
    pub async fn new_cluster(
        nodes: Vec<String>,
        broadcast_capacity: usize,
        conf: &RedisSettings,
    ) -> CustomResult<Self, errors::RedisError> {
        let (push_sender, _) =
            tokio::sync::broadcast::channel::<redis::PushInfo>(broadcast_capacity);

        let mut cluster_builder = redis::cluster::ClusterClient::builder(nodes.clone())
            .use_protocol(redis::ProtocolVersion::RESP3)
            .push_sender(push_sender.clone())
            .retries(conf.reconnect_max_attempts)
            .min_retry_wait(u64::from(conf.reconnect_delay))
            .response_timeout(std::time::Duration::from_secs(
                conf.default_command_timeout.max(1),
            ));

        if conf.max_in_flight_commands > 0 {
            let limit = usize::try_from(conf.max_in_flight_commands).unwrap_or_else(|_| {
                tracing::warn!(
                    "max_in_flight_commands ({}) exceeds usize, using usize::MAX",
                    conf.max_in_flight_commands
                );
                usize::MAX
            });
            cluster_builder = cluster_builder.connection_concurrency_limit(limit);
        }

        let cluster_client = cluster_builder
            .build()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let connection = cluster_client
            .get_async_connection()
            .await
            .change_context(errors::RedisError::RedisConnectionError)?;

        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(64);

        let push_receiver = push_sender.subscribe();

        let backend = SubscriberBackend::Cluster {
            connection,
        };

        tokio::spawn(Self::run(backend, push_receiver, broadcast_sender.clone(), command_receiver));

        Ok(Self {
            command_sender,
            broadcast_sender,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    /// Subscribe to a channel — waits for Redis to confirm the subscription
    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        let (done_sender, done_receiver) = tokio::sync::oneshot::channel::<()>();
        self.command_sender
            .send(SubscriberCommand::Subscribe {
                channel: channel.to_string(),
                done: done_sender,
            })
            .await
            .change_context(errors::RedisError::SubscribeError)?;
        done_receiver
            .await
            .change_context(errors::RedisError::SubscribeError)
    }

    /// Unsubscribe from a channel — waits for Redis to confirm
    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        let (done_sender, done_receiver) = tokio::sync::oneshot::channel::<()>();
        self.command_sender
            .send(SubscriberCommand::Unsubscribe {
                channel: channel.to_string(),
                done: done_sender,
            })
            .await
            .change_context(errors::RedisError::SubscribeError)?;
        done_receiver
            .await
            .change_context(errors::RedisError::SubscribeError)
    }

    /// Get a receiver for pub/sub messages
    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.broadcast_sender.subscribe()
    }

    pub fn is_subscriber_handler_spawned(&self) -> &Arc<atomic::AtomicBool> {
        &self.is_subscriber_handler_spawned
    }

    /// Background task: owns the connection, reads push messages, handles commands
    async fn run(
        mut backend: SubscriberBackend,
        mut push_receiver: tokio::sync::broadcast::Receiver<redis::PushInfo>,
        broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
        mut command_receiver: tokio::sync::mpsc::Receiver<SubscriberCommand>,
    ) {
        loop {
            tokio::select! {
                push_info = push_receiver.recv() => {
                    match push_info {
                        Ok(info) => {
                            Self::handle_push_info(&info, &broadcast_sender);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!(count, "Push receiver lagged — dropped messages");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::error!("Push channel closed — connection dropped permanently");
                            break;
                        }
                    }
                }
                command = command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            if let Err(error) = Self::handle_command(&mut backend, cmd).await {
                                tracing::error!(?error, "Failed to handle subscriber command");
                            }
                        }
                        None => {
                            // All command senders dropped — shutdown
                            break;
                        }
                    }
                }
            }
        }
    }

    fn handle_push_info(
        push_info: &redis::PushInfo,
        broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
    ) {
        // Only forward actual messages — skip subscribe/unsubscribe confirmations
        if push_info.kind != redis::PushKind::Message {
            return;
        }

        // PushInfo data for a message: [channel_name, payload]
        let channel = push_info
            .data
            .first()
            .map(|value| match value {
                Value::BulkString(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                Value::SimpleString(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let payload = push_info.data.get(1).cloned().unwrap_or(Value::Nil);

        if let Err(error) = broadcast_sender.send(PubSubMessage {
            channel,
            value: payload,
        }) {
            tracing::warn!(
                ?error,
                "Failed to broadcast pub/sub message — no active receivers"
            );
        }
    }

    async fn handle_command(
        backend: &mut SubscriberBackend,
        command: SubscriberCommand,
    ) -> CustomResult<(), errors::RedisError> {
        match command {
            SubscriberCommand::Subscribe { channel, done } => {
                let result = match backend {
                    SubscriberBackend::Standalone { connection } => {
                        let mut conn = connection.clone();
                        conn.subscribe(&channel).await
                    }
                    SubscriberBackend::Cluster { connection } => {
                        connection.subscribe(&channel).await
                    }
                };
                if done.send(()).is_err() {
                    tracing::warn!(
                        channel = %channel,
                        "Subscribe completed but caller already dropped the wait handle"
                    );
                }
                result.change_context(errors::RedisError::SubscribeError)?;
            }
            SubscriberCommand::Unsubscribe { channel, done } => {
                let result = match backend {
                    SubscriberBackend::Standalone { connection } => {
                        let mut conn = connection.clone();
                        conn.unsubscribe(&channel).await
                    }
                    SubscriberBackend::Cluster { connection } => {
                        connection.unsubscribe(&channel).await
                    }
                };
                if done.send(()).is_err() {
                    tracing::warn!(
                        channel = %channel,
                        "Unsubscribe completed but caller already dropped the wait handle"
                    );
                }
                result.change_context(errors::RedisError::SubscribeError)?;
            }
        }
        Ok(())
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
        Ok(Self {
            inner: RedisConn::Standalone(conn),
        })
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
        let (pool, subscriber, publisher) = match conf.cluster_enabled {
            true => {
                let nodes: Vec<String> = conf
                    .cluster_urls
                    .iter()
                    .map(|url| {
                        if url.starts_with("redis://") {
                            url.clone()
                        } else {
                            format!("redis://{url}")
                        }
                    })
                    .collect();

                let mut pool_builder = redis::cluster::ClusterClient::builder(nodes.clone())
                    .retries(conf.reconnect_max_attempts)
                    .min_retry_wait(u64::from(conf.reconnect_delay))
                    .response_timeout(std::time::Duration::from_secs(
                        conf.default_command_timeout.max(1),
                    ));

                if conf.max_in_flight_commands > 0 {
                    let limit = usize::try_from(conf.max_in_flight_commands).unwrap_or_else(|_| {
                        tracing::warn!(
                            "max_in_flight_commands ({}) exceeds usize, using usize::MAX",
                            conf.max_in_flight_commands
                        );
                        usize::MAX
                    });
                    pool_builder = pool_builder.connection_concurrency_limit(limit);
                }

                pool_builder = pool_builder.use_protocol(redis::ProtocolVersion::RESP3);

                let pool_conn = pool_builder
                    .build()
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

                let pool = RedisConn::Cluster(pool_conn);

                let mut publisher_builder = redis::cluster::ClusterClient::builder(nodes.clone())
                    .retries(conf.reconnect_max_attempts)
                    .min_retry_wait(u64::from(conf.reconnect_delay))
                    .response_timeout(std::time::Duration::from_secs(
                        conf.default_command_timeout.max(1),
                    ));

                publisher_builder =
                    publisher_builder.use_protocol(redis::ProtocolVersion::RESP3);

                let publisher_conn = publisher_builder
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

                let publisher = Arc::new(RedisClient {
                    inner: RedisConn::Cluster(publisher_conn),
                });

                let subscriber = Arc::new(
                    SubscriberClient::new_cluster(nodes, conf.broadcast_channel_capacity, conf)
                        .await?,
                );

                (pool, subscriber, publisher)
            }
            false => {
                let redis_connection_url = format!("redis://{}:{}", conf.host, conf.port);

                let mut connection_info = redis_connection_url
                    .as_str()
                    .into_connection_info()
                    .change_context(errors::RedisError::RedisConnectionError)?;

                let redis_settings = connection_info
                    .redis_settings()
                    .clone()
                    .set_protocol(redis::ProtocolVersion::RESP3);
                connection_info = connection_info.set_redis_settings(redis_settings);

                let client = redis::Client::open(connection_info)
                    .change_context(errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to open Redis client for {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let reconnection_retries =
                    usize::try_from(conf.reconnect_max_attempts).unwrap_or_else(|_| {
                        tracing::warn!(
                            "reconnect_max_attempts ({}) exceeds usize, using default (5)",
                            conf.reconnect_max_attempts
                        );
                        5
                    });
                let reconnection_min_delay =
                    std::time::Duration::from_millis(u64::from(conf.reconnect_delay));

                let mut pool_config = redis::aio::ConnectionManagerConfig::new()
                    .set_number_of_retries(reconnection_retries)
                    .set_min_delay(reconnection_min_delay);

                if conf.default_command_timeout > 0 {
                    pool_config = pool_config.set_response_timeout(
                        Some(std::time::Duration::from_secs(conf.default_command_timeout)),
                    );
                }

                if conf.max_in_flight_commands > 0 {
                    let pipeline_buffer_size =
                        usize::try_from(conf.max_in_flight_commands).unwrap_or_else(|_| {
                            tracing::warn!(
                                "max_in_flight_commands ({}) exceeds usize, using usize::MAX",
                                conf.max_in_flight_commands
                            );
                            usize::MAX
                        });
                    pool_config =
                        pool_config.set_pipeline_buffer_size(pipeline_buffer_size);
                }

                let conn = redis::aio::ConnectionManager::new_with_config(
                    client,
                    pool_config,
                )
                .await
                .change_context(errors::RedisError::RedisConnectionError)
                .attach_printable_lazy(|| {
                    format!("Failed to connect to Redis at {}:{}", conf.host, conf.port)
                })?;

                let pool = RedisConn::Standalone(conn);

                let mut base_connection_info = redis_connection_url
                    .as_str()
                    .into_connection_info()
                    .change_context(errors::RedisError::RedisConnectionError)?;

                let redis_settings = base_connection_info
                    .redis_settings()
                    .clone()
                    .set_protocol(redis::ProtocolVersion::RESP3);
                base_connection_info = base_connection_info.set_redis_settings(redis_settings);

                let base_client = redis::Client::open(base_connection_info)
                    .change_context(errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to open Redis pub/sub client for {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let subscriber = Arc::new(
                    SubscriberClient::new(
                        base_client.clone(),
                        conf.broadcast_channel_capacity,
                        Self::build_subscriber_config(conf),
                    )
                    .await?,
                );
                let publisher = Arc::new(RedisClient::new(&base_client).await?);

                (pool, subscriber, publisher)
            }
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

    fn build_subscriber_config(conf: &RedisSettings) -> redis::aio::ConnectionManagerConfig {
        let reconnection_retries =
            usize::try_from(conf.reconnect_max_attempts).unwrap_or_else(|_| {
                tracing::warn!(
                    "reconnect_max_attempts ({}) exceeds usize, using default (5)",
                    conf.reconnect_max_attempts
                );
                5
            });
        let reconnection_min_delay =
            std::time::Duration::from_millis(u64::from(conf.reconnect_delay));

        let mut config = redis::aio::ConnectionManagerConfig::new()
            .set_number_of_retries(reconnection_retries)
            .set_min_delay(reconnection_min_delay);

        if conf.default_command_timeout > 0 {
            config = config.set_response_timeout(
                Some(std::time::Duration::from_secs(conf.default_command_timeout)),
            );
        }

        config
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
