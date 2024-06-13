use error_stack::ResultExt;
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use router_env::{logger, tracing::Instrument};

use crate::redis::cache::{
    CacheKey, CacheKind, ACCOUNTS_CACHE, CGRAPH_CACHE, CONFIG_CACHE, DECISION_MANAGER_CACHE,
    PM_FILTERS_CGRAPH_CACHE, ROUTING_CACHE, SURCHARGE_CACHE,
};

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
impl PubSubInterface for std::sync::Arc<redis_interface::RedisConnectionPool> {
    #[inline]
    async fn subscribe(&self, channel: &str) -> error_stack::Result<(), redis_errors::RedisError> {
        // Spawns a task that will automatically re-subscribe to any channels or channel patterns used by the client.
        self.subscriber.manage_subscriptions();

        self.subscriber
            .subscribe(channel)
            .await
            .change_context(redis_errors::RedisError::SubscribeError)?;

        let redis_clone = self.clone();
        let _task_handle = tokio::spawn(
            async move {
                if let Err(pubsub_error) = redis_clone.on_message().await {
                    logger::error!(?pubsub_error);
                }
            }
            .in_current_span(),
        );
        Ok(())
    }

    #[inline]
    async fn publish<'a>(
        &self,
        channel: &str,
        key: CacheKind<'a>,
    ) -> error_stack::Result<usize, redis_errors::RedisError> {
        self.publisher
            .publish(channel, RedisValue::from(key).into_inner())
            .await
            .change_context(redis_errors::RedisError::SubscribeError)
    }

    #[inline]
    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError> {
        logger::debug!("Started on message: {:?}", self.key_prefix);
        let mut rx = self.subscriber.on_message();
        while let Ok(message) = rx.recv().await {
            logger::debug!("Invalidating {message:?}");
            let key = match CacheKind::try_from(RedisValue::new(message.value))
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
                    CONFIG_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::Accounts(key) => {
                    ACCOUNTS_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::CGraph(key) => {
                    CGRAPH_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::PmFiltersCGraph(key) => {
                    PM_FILTERS_CGRAPH_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;

                    key
                }
                CacheKind::Routing(key) => {
                    ROUTING_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::DecisionManager(key) => {
                    DECISION_MANAGER_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::Surcharge(key) => {
                    SURCHARGE_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    key
                }
                CacheKind::All(key) => {
                    CONFIG_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    ACCOUNTS_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    CGRAPH_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    PM_FILTERS_CGRAPH_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    ROUTING_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    DECISION_MANAGER_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;
                    SURCHARGE_CACHE
                        .remove(CacheKey {
                            key: key.to_string(),
                            prefix: self.key_prefix.clone(),
                        })
                        .await;

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
