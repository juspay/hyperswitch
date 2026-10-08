use std::sync::{atomic, Arc};

use error_stack::ResultExt;
use redis_interface::{errors as redis_errors, RedisValue};
use router_env::{logger, tracing::Instrument};

use crate::redis::cache::{CacheKey, CacheKind, CacheRedact, Caches};

#[async_trait::async_trait]
pub trait PubSubInterface {
    /// Subscribes to `channel`, spawning the handler that applies incoming invalidations to
    /// `caches` if it is not already running.
    async fn subscribe(
        &self,
        channel: &str,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError>;

    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError>;

    async fn on_message(
        &self,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for Arc<redis_interface::RedisConnectionPool> {
    #[inline]
    async fn subscribe(
        &self,
        channel: &str,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError> {
        self.subscriber.subscribe(channel).await?;

        // Spawn only one thread handling all the published messages to different channels.
        //
        // The handler is process-wide while stores are per tenant, so the caches it is given
        // must be the same set every tenant's store reads from — otherwise whichever tenant
        // subscribed first would be the only one whose entries ever get invalidated.
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
                    if let Err(pubsub_error) = redis_clone.on_message(caches).await {
                        logger::error!(?pubsub_error);
                    }
                }
                .in_current_span(),
            );
        }

        Ok(())
    }

    #[inline]
    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError> {
        let key = CacheRedact {
            kind: key,
            tenant: self.key_prefix.clone(),
        };

        self.publisher
            .publish(
                channel,
                RedisValue::try_from(key).change_context(redis_errors::RedisError::PublishError)?,
            )
            .await
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
    async fn on_message(
        &self,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError> {
        logger::debug!("Started on message");
        let mut rx = self.subscriber.message_rx();
        while let Ok(message) = rx.recv().await {
            let channel_name = message.channel.to_string();
            logger::debug!("Received message on channel: {channel_name}");

            if channel_name != caches.invalidation_channel {
                logger::debug!("Received message from unknown channel: {channel_name}");
                continue;
            }

            let message = match CacheRedact::try_from(message.value)
                .change_context(redis_errors::RedisError::OnMessageError)
            {
                Ok(value) => value,
                Err(err) => {
                    logger::error!(value_conversion_err=?err);
                    continue;
                }
            };

            let key = CacheKey {
                key: message.kind.get_key_without_prefix().to_owned(),
                prefix: message.tenant.clone(),
            };
            for cache in caches.for_kind(&message.kind) {
                cache.remove(key.clone()).await;
            }

            logger::debug!(
                key_prefix=?message.tenant,
                channel_name=?channel_name,
                "Done invalidating {}",
                key.key
            );
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubSubInterface for redis_interface::RedisConnectionWithContext {
    async fn subscribe(
        &self,
        channel: &str,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError> {
        self.redis_conn.subscribe(channel, caches).await
    }

    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError> {
        self.redis_conn.publish(channel, key).await
    }

    async fn on_message(
        &self,
        caches: Arc<Caches>,
    ) -> error_stack::Result<(), redis_errors::RedisError> {
        self.redis_conn.on_message(caches).await
    }
}
