use std::sync::atomic;

use error_stack::ResultExt;
use redis_interface::{errors as redis_errors, PubsubInterface, RedisValue};
use router_env::{logger, tracing::Instrument};

use crate::redis::cache::{
    CacheKey, CacheKind, CacheRedact, ACCOUNTS_CACHE, CGRAPH_CACHE, CONFIG_CACHE,
    CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE, DECISION_MANAGER_CACHE,
    ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE, PM_FILTERS_CGRAPH_CACHE, ROUTING_CACHE,
    SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE, SURCHARGE_CACHE,
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
            .subscribe::<(), &str>(channel)
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
    async fn on_message(&self) -> error_stack::Result<(), redis_errors::RedisError> {
        logger::debug!("Started on message");
        let mut rx = self.subscriber.on_message();
        while let Ok(message) = rx.recv().await {
            let channel_name = message.channel.to_string();
            logger::debug!("Received message on channel: {channel_name}");

            match channel_name.as_str() {
                super::cache::IMC_INVALIDATION_CHANNEL => {
                    let message = match CacheRedact::try_from(RedisValue::new(message.value))
                        .change_context(redis_errors::RedisError::OnMessageError)
                    {
                        Ok(value) => value,
                        Err(err) => {
                            logger::error!(value_conversion_err=?err);
                            continue;
                        }
                    };

                    let key = match message.kind {
                        CacheKind::Config(key) => {
                            CONFIG_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::Accounts(key) => {
                            ACCOUNTS_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::CGraph(key) => {
                            CGRAPH_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::PmFiltersCGraph(key) => {
                            PM_FILTERS_CGRAPH_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::EliminationBasedDynamicRoutingCache(key) => {
                            ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::ContractBasedDynamicRoutingCache(key) => {
                            CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::SuccessBasedDynamicRoutingCache(key) => {
                            SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::Routing(key) => {
                            ROUTING_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::DecisionManager(key) => {
                            DECISION_MANAGER_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::Surcharge(key) => {
                            SURCHARGE_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            key
                        }
                        CacheKind::All(key) => {
                            CONFIG_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            ACCOUNTS_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            CGRAPH_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            PM_FILTERS_CGRAPH_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            ROUTING_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            DECISION_MANAGER_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;
                            SURCHARGE_CACHE
                                .remove(CacheKey {
                                    key: key.to_string(),
                                    prefix: message.tenant.clone(),
                                })
                                .await;

                            key
                        }
                    };

                    logger::debug!(
                        key_prefix=?message.tenant.clone(),
                        channel_name=?channel_name,
                        "Done invalidating {key}"
                    );
                }
                _ => {
                    logger::debug!("Received message from unknown channel: {channel_name}");
                }
            }
        }
        Ok(())
    }
}
