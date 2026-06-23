//! Redis-rs backend implementation for the redis_interface crate.
//!
//! This module contains the connection pool, subscriber, and publisher
//! implementations using the `redis` crate (redis-rs).

pub mod commands;
pub mod types;

use std::sync::{atomic, Arc};

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use redis::AsyncCommands;
use tracing::Instrument;
pub use types::redis_value_to_option_string;

// ─── Cluster abstraction ────────────────────────────────────────────────────

/// An abstraction over standalone and cluster Redis connections.
/// Both variants implement the same async command interface, so all
/// Redis operations work transparently on either.
#[derive(Clone)]
pub enum RedisConn {
    Standalone(redis::aio::ConnectionManager),
    Cluster(redis::cluster_async::ClusterConnection),
}

impl redis::aio::ConnectionLike for RedisConn {
    fn req_packed_command<'a>(
        &'a mut self,
        cmd: &'a redis::Cmd,
    ) -> redis::RedisFuture<'a, redis::Value> {
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
    ) -> redis::RedisFuture<'a, Vec<redis::Value>> {
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
/// Subscribe/unsubscribe calls go directly to the Redis connection.
/// A background task reads push messages and forwards them to a broadcast channel.
pub struct SubscriberClient {
    connection: SubscriberBackend,
    broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
    pub is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
}

enum SubscriberBackend {
    Standalone {
        connection: redis::aio::ConnectionManager,
    },
    Cluster {
        // Mutex required because ClusterConnection::subscribe/unsubscribe
        // needs &mut self, unlike ConnectionManager which can be cloned
        connection: tokio::sync::Mutex<redis::cluster_async::ClusterConnection>,
    },
}

/// Represents a message received from a pub/sub channel.
#[derive(Clone, Debug)]
pub struct PubSubMessage {
    pub channel: String,
    pub value: crate::types::RedisValue,
}

impl PubSubMessage {
    /// Attempts to extract a [`PubSubMessage`] from a RESP3 push info.
    ///
    /// Returns `None` if the push kind is not `Message`, or if the
    /// channel name or payload cannot be extracted from the data.
    fn from_push_info(push_info: &redis::PushInfo) -> Option<Self> {
        if push_info.kind != redis::PushKind::Message {
            return None;
        }

        // RESP3 pub/sub message format:
        //   data[0] = channel name (string)
        //   data[1] = message payload (any Value)
        let channel_value = push_info.data.first()?;
        let channel = match channel_value {
            redis::Value::BulkString(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            redis::Value::SimpleString(string) => string.clone(),
            redis::Value::VerbatimString { text, .. } => text.clone(),
            // Non-string variants are not valid channel names in the RESP3
            // pub/sub protocol. Reject the message rather than fabricating
            // a default channel name.
            redis::Value::Nil
            | redis::Value::Int(_)
            | redis::Value::Okay
            | redis::Value::Double(_)
            | redis::Value::Boolean(_)
            | redis::Value::BigNumber(_)
            | redis::Value::Array(_)
            | redis::Value::Map(_)
            | redis::Value::Set(_)
            | redis::Value::Attribute { .. }
            | redis::Value::Push { .. }
            | redis::Value::ServerError(_) => {
                tracing::warn!(
                    ?channel_value,
                    "Pub/sub channel name is not a string variant — malformed push data"
                );
                return None;
            }
            // Catch-all for future variants added to the non-exhaustive enum
            _ => {
                tracing::warn!(
                    ?channel_value,
                    "Unknown Value variant in pub/sub channel name — malformed push data"
                );
                return None;
            }
        };

        if channel.is_empty() {
            tracing::warn!("Pub/sub channel name is empty — malformed push data");
            return None;
        }

        let payload = match push_info.data.get(1) {
            Some(value) => value.clone(),
            None => {
                tracing::warn!(
                    ?push_info,
                    "Pub/sub message has no payload — malformed push data"
                );
                return None;
            }
        };

        Some(Self {
            channel,
            value: crate::types::RedisValue::new(payload),
        })
    }
}

impl SubscriberClient {
    pub(crate) async fn new(
        conf: &crate::types::RedisSettings,
    ) -> CustomResult<Self, crate::errors::RedisError> {
        let (push_sender, push_receiver) =
            tokio::sync::broadcast::channel::<redis::PushInfo>(conf.broadcast_channel_capacity);

        let connection = match conf.cluster_enabled {
            true => Self::create_cluster_backend(conf, push_sender.clone()).await?,
            false => Self::create_standalone_backend(conf, push_sender.clone()).await?,
        };

        let (broadcast_sender, _) =
            tokio::sync::broadcast::channel(conf.broadcast_channel_capacity);

        tokio::spawn(Self::run(push_receiver, broadcast_sender.clone()).in_current_span());

        Ok(Self {
            connection,
            broadcast_sender,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
        })
    }

    async fn create_standalone_backend(
        conf: &crate::types::RedisSettings,
        push_sender: tokio::sync::broadcast::Sender<redis::PushInfo>,
    ) -> CustomResult<SubscriberBackend, crate::errors::RedisError> {
        let connection_info = conf.build_standalone_connection_info()?;

        let redis_client = redis::Client::open(connection_info)
            .change_context(crate::errors::RedisError::RedisConnectionError)?;

        let config = conf
            .build_connection_manager_config()
            .set_push_sender(push_sender)
            .set_automatic_resubscription();

        let connection = redis::aio::ConnectionManager::new_with_config(redis_client, config)
            .await
            .change_context(crate::errors::RedisError::RedisConnectionError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to create subscriber connection for {}:{}",
                    conf.host, conf.port
                )
            })?;

        Ok(SubscriberBackend::Standalone { connection })
    }

    async fn create_cluster_backend(
        conf: &crate::types::RedisSettings,
        push_sender: tokio::sync::broadcast::Sender<redis::PushInfo>,
    ) -> CustomResult<SubscriberBackend, crate::errors::RedisError> {
        let nodes = conf.normalize_cluster_urls();

        let mut cluster_builder = conf
            .build_cluster_client_builder(nodes)
            .push_sender(push_sender);

        if conf.max_in_flight_commands > 0 {
            cluster_builder =
                cluster_builder.connection_concurrency_limit(conf.max_in_flight_commands);
        }

        let connection = cluster_builder
            .build()
            .change_context(crate::errors::RedisError::RedisConnectionError)?
            .get_async_connection()
            .await
            .change_context(crate::errors::RedisError::RedisConnectionError)?;

        Ok(SubscriberBackend::Cluster {
            connection: tokio::sync::Mutex::new(connection),
        })
    }

    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), crate::errors::RedisError> {
        match &self.connection {
            SubscriberBackend::Standalone { connection } => connection
                .clone()
                .subscribe(channel)
                .await
                .change_context(crate::errors::RedisError::SubscribeError),
            SubscriberBackend::Cluster { connection } => connection
                .lock()
                .await
                .subscribe(channel)
                .await
                .change_context(crate::errors::RedisError::SubscribeError),
        }
    }

    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), crate::errors::RedisError> {
        match &self.connection {
            SubscriberBackend::Standalone { connection } => connection
                .clone()
                .unsubscribe(channel)
                .await
                .change_context(crate::errors::RedisError::SubscribeError),
            SubscriberBackend::Cluster { connection } => connection
                .lock()
                .await
                .unsubscribe(channel)
                .await
                .change_context(crate::errors::RedisError::SubscribeError),
        }
    }

    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.broadcast_sender.subscribe()
    }

    /// Background task that reads RESP3 push messages from a broadcast receiver
    /// and forwards them as [`PubSubMessage`]s to an internal broadcast channel.
    ///
    /// The loop exits when the push channel is closed (i.e. the underlying
    /// Redis connection has dropped permanently). When that happens, the
    /// subscriber's `broadcast_sender` remains functional — callers invoking
    /// [`message_rx()`](Self::message_rx) will simply never receive new messages.
    async fn run(
        mut push_receiver: tokio::sync::broadcast::Receiver<redis::PushInfo>,
        broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
    ) {
        loop {
            match push_receiver.recv().await {
                Ok(info) => Self::handle_push_info(&info, &broadcast_sender),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(count, "Push receiver lagged — dropped messages");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::error!("Push channel closed — connection dropped permanently");
                    break;
                }
            }
        }
        tracing::warn!("Subscriber handler task exiting — no further messages will be received");
    }

    /// Parses a single push message and, if it is a pub/sub `Message`,
    /// broadcasts it to all active receivers. Non-message push kinds
    /// (e.g. `Subscribe`, `Unsubscribe`) are logged at warn level and ignored.
    fn handle_push_info(
        push_info: &redis::PushInfo,
        broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
    ) {
        match PubSubMessage::from_push_info(push_info) {
            Some(msg) => {
                if let Err(error) = broadcast_sender.send(msg) {
                    tracing::error!(
                        ?error,
                        "Failed to broadcast pub/sub message — no active receivers"
                    );
                }
            }
            None => {
                tracing::warn!(
                    kind = ?push_info.kind,
                    "Ignoring non-message push kind"
                );
            }
        }
    }
}

// ─── Publisher client ────────────────────────────────────────────────────────

/// A wrapper around a connection used for publishing messages.
pub struct PublisherClient {
    inner: RedisConn,
}

impl PublisherClient {
    pub(crate) fn new(connection: RedisConn) -> Self {
        Self { inner: connection }
    }

    pub async fn publish(
        &self,
        channel: &str,
        message: crate::types::RedisValue,
    ) -> CustomResult<usize, crate::errors::RedisError> {
        let mut conn = self.inner.clone();
        conn.publish::<_, _, usize>(channel, message)
            .await
            .change_context(crate::errors::RedisError::PublishError)
    }
}

// ─── Connection pool ─────────────────────────────────────────────────────────

pub struct RedisConnectionPool {
    pub pool: RedisConn,
    pub key_prefix: String,
    pub config: Arc<RedisConfig>,
    pub subscriber: Arc<SubscriberClient>,
    pub publisher: Arc<PublisherClient>,
    pub is_redis_available: Arc<atomic::AtomicBool>,
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    pub async fn new(
        conf: &crate::types::RedisSettings,
    ) -> CustomResult<Self, crate::errors::RedisError> {
        let (pool, subscriber, publisher) = match conf.cluster_enabled {
            true => {
                let nodes = conf.normalize_cluster_urls();

                let mut pool_builder = conf.build_cluster_client_builder(nodes.clone());

                if conf.max_in_flight_commands > 0 {
                    pool_builder =
                        pool_builder.connection_concurrency_limit(conf.max_in_flight_commands);
                }

                let pool_conn = pool_builder
                    .build()
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to create Redis cluster client for {}:{}",
                            conf.host, conf.port
                        )
                    })?
                    .get_async_connection()
                    .await
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to connect to Redis cluster at {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let pool = RedisConn::Cluster(pool_conn);

                let publisher_builder = conf.build_cluster_client_builder(nodes);

                let publisher_conn = publisher_builder
                    .build()
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to create Redis cluster pub/sub client for {}:{}",
                            conf.host, conf.port
                        )
                    })?
                    .get_async_connection()
                    .await
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to connect Redis cluster pub/sub at {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let publisher = Arc::new(PublisherClient::new(RedisConn::Cluster(publisher_conn)));

                let subscriber = Arc::new(SubscriberClient::new(conf).await?);

                (pool, subscriber, publisher)
            }
            false => {
                let connection_info = conf.build_standalone_connection_info()?;

                let client = redis::Client::open(connection_info)
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to open Redis client for {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let mut pool_config = conf.build_connection_manager_config();

                if conf.max_in_flight_commands > 0 {
                    pool_config = pool_config.set_pipeline_buffer_size(conf.max_in_flight_commands);
                }

                let conn = redis::aio::ConnectionManager::new_with_config(client, pool_config)
                    .await
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!("Failed to connect to Redis at {}:{}", conf.host, conf.port)
                    })?;

                let pool = RedisConn::Standalone(conn);

                let base_connection_info = conf.build_standalone_connection_info()?;

                let base_client = redis::Client::open(base_connection_info)
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to open Redis pub/sub client for {}:{}",
                            conf.host, conf.port
                        )
                    })?;

                let subscriber = Arc::new(SubscriberClient::new(conf).await?);

                let publisher_conn = redis::aio::ConnectionManager::new(base_client)
                    .await
                    .change_context(crate::errors::RedisError::RedisConnectionError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to create publisher connection for {}:{}",
                            conf.host, conf.port
                        )
                    })?;
                let publisher =
                    Arc::new(PublisherClient::new(RedisConn::Standalone(publisher_conn)));

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
    /// When Redis is unreachable for longer than `max_failure_threshold_seconds` seconds,
    /// signals via the oneshot sender and marks redis as unavailable.
    pub async fn on_error(&self, tx: tokio::sync::oneshot::Sender<()>) {
        let check_interval = self
            .config
            .unresponsive_check_interval
            .max(crate::constant::redis_rs_commands::MIN_ERROR_CHECK_INTERVAL_SECS);
        let max_unreachable_secs = self.config.max_failure_threshold_seconds;
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
            } else {
                let now = std::time::Instant::now();
                let first_failure = *first_failure_at.get_or_insert(now);
                let unreachable_secs = now.duration_since(first_failure).as_secs();

                if unreachable_secs >= u64::from(max_unreachable_secs) {
                    tracing::error!(
                        "Redis has been unreachable for {}s (threshold: {}s), shutting down",
                        unreachable_secs,
                        max_unreachable_secs
                    );
                    if let Err(error) = tx.send(()) {
                        tracing::warn!(
                            ?error,
                            "Failed to send shutdown signal — receiver already dropped"
                        );
                    }
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
    }
}

// ─── RedisSettings helpers (redis-rs backend only) ────────────────────────────

impl crate::types::RedisSettings {
    /// Normalize cluster URLs by prepending `"redis://"` if the scheme is missing.
    pub(crate) fn normalize_cluster_urls(&self) -> Vec<String> {
        self.cluster_urls
            .iter()
            .map(|url| {
                if url.starts_with("redis://") {
                    url.clone()
                } else {
                    format!("redis://{url}")
                }
            })
            .collect()
    }

    /// Convert `reconnect_max_attempts` to `usize`.
    ///
    /// Logs a warning and falls back to [`crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS`]
    /// when the value overflows.
    pub(crate) fn reconnect_max_attempts_as_usize(&self) -> usize {
        usize::try_from(self.reconnect_max_attempts).unwrap_or_else(|_| {
            tracing::warn!(
                "reconnect_max_attempts ({}) exceeds usize, using default ({})",
                self.reconnect_max_attempts,
                crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS
            );
            crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS
        })
    }

    /// Build standalone connection info with RESP3 protocol from host and port.
    pub(crate) fn build_standalone_connection_info(
        &self,
    ) -> CustomResult<redis::ConnectionInfo, crate::errors::RedisError> {
        use error_stack::ResultExt;
        use redis::IntoConnectionInfo;

        let connection_url = format!("redis://{}:{}", self.host, self.port);
        let mut connection_info = connection_url
            .as_str()
            .into_connection_info()
            .change_context(crate::errors::RedisError::RedisConnectionError)?;

        let redis_settings = connection_info
            .redis_settings()
            .clone()
            .set_protocol(redis::ProtocolVersion::RESP3);
        connection_info = connection_info.set_redis_settings(redis_settings);

        Ok(connection_info)
    }

    /// Build the connection manager configuration from these settings.
    ///
    /// Sets reconnection retries, minimum delay, and optional response timeout.
    pub(crate) fn build_connection_manager_config(&self) -> redis::aio::ConnectionManagerConfig {
        let mut config = redis::aio::ConnectionManagerConfig::new()
            .set_number_of_retries(self.reconnect_max_attempts_as_usize())
            .set_min_delay(std::time::Duration::from_millis(u64::from(
                self.reconnect_delay,
            )));

        if self.default_command_timeout > 0 {
            config = config.set_response_timeout(Some(std::time::Duration::from_secs(
                self.default_command_timeout,
            )));
        }

        config
    }

    /// Build a cluster client builder with common configuration.
    ///
    /// Sets retries, retry wait, response timeout, and RESP3 protocol.
    pub(crate) fn build_cluster_client_builder(
        &self,
        nodes: Vec<String>,
    ) -> redis::cluster::ClusterClientBuilder {
        redis::cluster::ClusterClient::builder(nodes)
            .retries(self.reconnect_max_attempts)
            .min_retry_wait(u64::from(self.reconnect_delay))
            .response_timeout(std::time::Duration::from_secs(
                self.default_command_timeout.max(1),
            ))
            .use_protocol(redis::ProtocolVersion::RESP3)
    }
}

pub struct RedisConfig {
    pub(crate) default_ttl: u32,
    pub(crate) default_stream_read_count: u64,
    pub(crate) default_hash_ttl: u32,
    pub(crate) cluster_enabled: bool,
    pub(crate) unresponsive_check_interval: u64,
    pub(crate) max_failure_threshold_seconds: u32,
}

impl From<&crate::types::RedisSettings> for RedisConfig {
    fn from(config: &crate::types::RedisSettings) -> Self {
        Self {
            default_ttl: config.default_ttl,
            default_stream_read_count: config.stream_read_count,
            default_hash_ttl: config.default_hash_ttl,
            cluster_enabled: config.cluster_enabled,
            unresponsive_check_interval: config.unresponsive_check_interval,
            max_failure_threshold_seconds: config.max_failure_threshold_seconds,
        }
    }
}
