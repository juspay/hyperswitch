use error_stack::{IntoReport, ResultExt};
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use router_env::logger;

use crate::redis::cache::{CacheKind, ACCOUNTS_CACHE, CONFIG_CACHE};

#[async_trait::async_trait]
pub trait PubSubInterface {
    async fn subscribe(&self, channel: &str) -> error_stack::Result<(), redis_errors::RedisError>;

    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError>;

    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for redis_interface::RedisConnectionPool {
    #[inline]
        /// Asynchronously subscribes to the specified channel and automatically re-subscribes to any channels or channel patterns used by the client. Returns a Result indicating success or a RedisError if there is an issue with the subscription process.
    async fn subscribe(&self, channel: &str) -> error_stack::Result<(), redis_errors::RedisError> {
        // Spawns a task that will automatically re-subscribe to any channels or channel patterns used by the client.
        self.subscriber.manage_subscriptions();

        self.subscriber
            .subscribe(channel)
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
        /// Asynchronously publishes a message to a specified channel using the given key, and returns the number of subscribers who received the message.
    ///
    /// # Arguments
    ///
    /// * `channel` - A reference to a string representing the channel to publish the message to.
    /// * `key` - The cache key to be used for publishing the message.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of subscribers who received the message, or a `RedisError` if an error occurs during the publishing process.
    ///
    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError> {
        self.publisher
            .publish(channel, RedisValue::from(key).into_inner())
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
        /// Asynchronously listens for messages on a Redis subscriber channel and invalidates cache keys based on the received messages. 
    /// Returns a Result indicating success or an error of type `redis_errors::RedisError`.
    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError> {
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
