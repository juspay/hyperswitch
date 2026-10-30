use std::{
    future::Future,
    sync::{atomic, Arc},
};

use error_stack::ResultExt;
use redis_interface::{errors as redis_errors, PubSubMessage, RedisValue};
use router_env::{logger, tracing::Instrument};
use tokio::sync::broadcast;

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
        let redis_clone = self.clone();
        let _task_handle =
            spawn_handler_once(&self.subscriber.is_subscriber_handler_spawned, async move {
                if let Err(pubsub_error) = redis_clone.on_message(caches).await {
                    logger::error!(?pubsub_error);
                }
            });

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
        apply_invalidations(self.subscriber.message_rx(), &caches).await;
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

/// Spawns `handler` unless the one tracked by `spawned` is still running.
///
/// `spawned` is cleared once the handler stops, however it stops, so that a later call spawns
/// a new one instead of assuming it still runs.
fn spawn_handler_once(
    spawned: &Arc<atomic::AtomicBool>,
    handler: impl Future<Output = ()> + Send + 'static,
) -> Option<tokio::task::JoinHandle<()>> {
    spawned
        .compare_exchange(
            false,
            true,
            atomic::Ordering::SeqCst,
            atomic::Ordering::SeqCst,
        )
        .ok()?;

    let spawned_guard = HandlerSpawnedGuard(Arc::clone(spawned));
    Some(tokio::spawn(
        async move {
            let _spawned_guard = spawned_guard;
            handler.await;
        }
        .in_current_span(),
    ))
}

/// Clears the handler-spawned flag when dropped, including when the handler panics.
struct HandlerSpawnedGuard(Arc<atomic::AtomicBool>);

impl Drop for HandlerSpawnedGuard {
    fn drop(&mut self) {
        self.0.store(false, atomic::Ordering::SeqCst);
    }
}

/// Applies the invalidations received on `rx` to `caches` until the channel is closed.
async fn apply_invalidations(mut rx: broadcast::Receiver<PubSubMessage>, caches: &Caches) {
    loop {
        let message = match rx.recv().await {
            Ok(message) => message,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                // The receiver is still usable, but the skipped messages may have been
                // invalidations, and nothing tells which entries they were for.
                logger::warn!(
                    skipped,
                    "Pub/sub receiver lagged, clearing the in-memory caches"
                );
                caches.invalidate_all();
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };

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
    logger::warn!("Pub/sub channel closed, no further invalidations will be applied");
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, time::Duration};

    use tokio::sync::oneshot;

    use super::*;

    const TENANT: &str = "tenant";

    fn config_key(key: &str) -> CacheKey {
        CacheKey {
            key: key.to_string(),
            prefix: TENANT.to_string(),
        }
    }

    /// The message `publish` sends to invalidate `key` in the config cache.
    fn config_invalidation(caches: &Caches, key: &str) -> PubSubMessage {
        let redact = CacheRedact {
            tenant: TENANT.to_string(),
            kind: CacheKind::Config(Cow::Owned(key.to_string())),
        };
        PubSubMessage {
            channel: caches.invalidation_channel.clone(),
            value: RedisValue::try_from(redact).expect("invalidation should serialize"),
        }
    }

    async fn is_cached(caches: &Caches, key: &str) -> bool {
        caches
            .config
            .get_val::<String>(config_key(key))
            .await
            .is_some()
    }

    /// Gives the handler time to catch up, failing the test if `key` is never invalidated.
    async fn wait_until_invalidated(caches: &Caches, key: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while is_cached(caches, key).await {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("`{key}` was never invalidated"));
    }

    #[tokio::test]
    async fn invalidations_dropped_by_a_lag_clear_the_caches_and_later_ones_still_apply() {
        let caches = Arc::new(Caches::default());
        caches
            .config
            .push(config_key("stale"), "value".to_string())
            .await;

        let (tx, rx) = broadcast::channel(2);
        let handler = tokio::spawn({
            let caches = Arc::clone(&caches);
            async move { apply_invalidations(rx, &caches).await }
        });

        // Three messages for a capacity of two, sent before the handler first runs: the
        // receiver lags past the oldest one, the only invalidation of `stale`.
        for key in ["stale", "other", "another"] {
            tx.send(config_invalidation(&caches, key))
                .expect("the handler holds a receiver");
        }
        wait_until_invalidated(&caches, "stale").await;

        // Cached after the lag was handled, so only the message below can invalidate it.
        caches
            .config
            .push(config_key("fresh"), "value".to_string())
            .await;
        assert!(is_cached(&caches, "fresh").await);
        tx.send(config_invalidation(&caches, "fresh"))
            .expect("the handler holds a receiver");
        wait_until_invalidated(&caches, "fresh").await;

        drop(tx);
        tokio::time::timeout(Duration::from_secs(5), handler)
            .await
            .expect("the handler should stop once the channel is closed")
            .expect("the handler should not panic");
    }

    #[tokio::test]
    async fn handler_is_spawned_again_once_the_previous_one_stops() {
        let spawned = Arc::new(atomic::AtomicBool::new(false));
        let (stop, stopped) = oneshot::channel::<()>();

        let first = spawn_handler_once(&spawned, async move {
            let _ = stopped.await;
        })
        .expect("no handler is running yet");
        assert!(spawn_handler_once(&spawned, async {}).is_none());

        drop(stop);
        first.await.expect("the handler should not panic");
        assert!(spawn_handler_once(&spawned, async {}).is_some());
    }

    #[tokio::test]
    async fn handler_is_spawned_again_after_the_previous_one_panics() {
        let spawned = Arc::new(atomic::AtomicBool::new(false));

        let first = spawn_handler_once(&spawned, async { panic!("handler failed") })
            .expect("no handler is running yet");
        assert!(first.await.expect_err("the handler panics").is_panic());
        assert!(spawn_handler_once(&spawned, async {}).is_some());
    }
}
