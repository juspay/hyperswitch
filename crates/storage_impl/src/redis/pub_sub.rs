use std::sync::atomic;

use error_stack::ResultExt;
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use router_env::{logger, tracing::Instrument};

use crate::redis::cache::{self, CacheKind};

#[async_trait::async_trait]
pub trait PubSubInterface {
    async fn subscribe(&self, channel: &str) -> error_stack::Result<(), redis_errors::RedisError>;

    async fn publish<'a, K>(
        &self,
        channel: &str,
        keys: K,
    ) -> error_stack::Result<usize, redis_errors::RedisError>
    where
        K: IntoIterator<Item = CacheKind<'a>> + Send;

    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for std::sync::Arc<redis_interface::RedisConnectionPool> {
    #[inline]
    async fn subscribe(&self, channel: &str) -> error_stack::Result<(), redis_errors::RedisError> {
        // Spawns a task that will automatically re-subscribe to any channels or channel patterns used by the client.
        self.subscriber.manage_subscriptions();

        self.subscriber
            .subscribe(channel)
            .await
            .change_context(redis_errors::RedisError::SubscribeError)?;

        // Spawn only one thread handling all the published messages to different channels
        if self
            .subscriber
            .is_subscriber_handler_spawned
            .compare_exchange(
                false,
                true,
                atomic::Ordering::SeqCst,
                atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            let redis_clone = self.clone();
            let _task_handle = tokio::spawn(
                async move {
                    if let Err(pubsub_error) = redis_clone.on_message().await {
                        logger::error!(?pubsub_error);
                    }
                }
                .in_current_span(),
            );
        }

        Ok(())
    }

    #[inline]
    async fn publish<'a, K>(
        &self,
        channel: &str,
        keys: K,
    ) -> error_stack::Result<usize, redis_errors::RedisError>
    where
        K: IntoIterator<Item = CacheKind<'a>> + Send,
    {
        let mut keys_to_be_published_to_redis = Vec::new();
        for key in keys {
            keys_to_be_published_to_redis.push(
                RedisValue::from(key)
                    .as_string()
                    .ok_or(redis_errors::RedisError::PublishError)
                    .attach_printable("Failed to convert RedisValue to String")?,
            )
        }
        let serialized_keys = serde_json::to_string(&keys_to_be_published_to_redis)
            .change_context(redis_errors::RedisError::JsonSerializationFailed).attach_printable("Failed while serializing keys to be published to IMC invalidation channel as a String of json")?;

        self.publisher
            .publish(
                channel,
                RedisValue::from_string(serialized_keys).into_inner(),
            )
            .await
            .change_context(redis_errors::RedisError::PublishError)
    }

    #[inline]
    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError> {
        logger::debug!("Started on message: {:?}", self.key_prefix);
        let mut rx = self.subscriber.on_message();
        while let Ok(message) = rx.recv().await {
            let channel_name = message.channel.to_string();
            logger::debug!("Received message on channel: {channel_name}");

            match channel_name.as_str() {
                cache::IMC_INVALIDATION_CHANNEL => {
                    let keys_to_be_invalidated = match CacheKind::from_redis_value(RedisValue::new(
                        message.value,
                    ))
                    .change_context(redis_errors::RedisError::OnMessageError)
                    {
                        Ok(value) => value,
                        Err(err) => {
                            logger::error!(value_conversion_err=?err, "Failed to parse the message on invalidation channel to CacheKind");
                            continue;
                        }
                    };

                    cache::invalidate_cache_entries(
                        keys_to_be_invalidated,
                        self.key_prefix.clone(),
                    )
                    .await;
                }
                _ => {
                    logger::debug!("Received message from unknown channel: {channel_name}");
                }
            }
        }
        Ok(())
    }
}
