pub mod cache;
pub mod kv_store;
pub mod pub_sub;

use std::sync::{atomic, Arc};

use error_stack::{IntoReport, ResultExt};
use redis_interface::PubsubInterface;
use router_env::logger;

use self::{kv_store::RedisConnInterface, pub_sub::PubSubInterface};

#[derive(Clone)]
pub struct RedisStore {
    // Maybe expose the redis_conn via traits instead of the making the field public
    pub(crate) redis_conn: Arc<redis_interface::RedisConnectionPool>,
}

impl std::fmt::Debug for RedisStore {
        /// This method formats the CacheStore struct for display, using the given formatter.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheStore")
            .field("redis_conn", &"Redis conn doesn't implement debug")
            .finish()
    }
}

impl RedisStore {
        /// Asynchronously creates a new instance of the RedisManager struct with the provided Redis settings.
    ///
    /// # Arguments
    ///
    /// * `conf` - A reference to a RedisSettings object containing the configuration for the Redis connection.
    ///
    /// # Returns
    ///
    /// A Result containing the newly created RedisManager wrapped in Ok, or a RedisError wrapped in Err if an error occurs during the creation process.
    pub async fn new(
        conf: &redis_interface::RedisSettings,
    ) -> error_stack::Result<Self, redis_interface::errors::RedisError> {
        Ok(Self {
            redis_conn: Arc::new(redis_interface::RedisConnectionPool::new(conf).await?),
        })
    }

        /// Sets the error callback for the Redis connection. When an error occurs, the provided
    /// callback will be invoked with a oneshot::Result indicating the error. The callback
    /// is executed in a separate Tokio task to ensure it does not block the main thread.
    pub fn set_error_callback(&self, callback: tokio::sync::oneshot::Sender<()>) {
        let redis_clone = self.redis_conn.clone();
        tokio::spawn(async move {
            redis_clone.on_error(callback).await;
        });
    }

        /// Asynchronously subscribes to a specified channel using the Redis connection. It manages the subscriptions, subscribes to the channel, and handles any errors that may occur during the subscription process. Additionally, it spawns a separate asynchronous task to handle incoming messages from the subscribed channel.
    pub async fn subscribe_to_channel(
        &self,
        channel: &str,
    ) -> error_stack::Result<(), redis_interface::errors::RedisError> {
        self.redis_conn.subscriber.manage_subscriptions();

        self.redis_conn
            .subscriber
            .subscribe::<(), _>(channel)
            .await
            .into_report()
            .change_context(redis_interface::errors::RedisError::SubscribeError)?;

        let redis_clone = self.redis_conn.clone();
        tokio::spawn(async move {
            if let Err(e) = redis_clone.on_message().await {
                logger::error!(pubsub_err=?e);
            }
        });
        Ok(())
    }
}

impl RedisConnInterface for RedisStore {
        /// Retrieves a Redis connection from the Redis connection pool if it is available.
    /// If the Redis connection is available, it returns a clone of the connection pool.
    /// If the Redis connection is not available, it returns a RedisConnectionError.
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<
        Arc<redis_interface::RedisConnectionPool>,
        redis_interface::errors::RedisError,
    > {
        if self
            .redis_conn
            .is_redis_available
            .load(atomic::Ordering::SeqCst)
        {
            Ok(self.redis_conn.clone())
        } else {
            Err(redis_interface::errors::RedisError::RedisConnectionError.into())
        }
    }
}
