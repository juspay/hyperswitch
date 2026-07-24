pub mod cache;
pub mod kv_store;
pub mod pub_sub;

use std::sync::{atomic, Arc};

use common_utils::external_service::{ExternalServiceEventEmitter, NoOpEventEmitter};
use router_env::tracing::Instrument;

#[derive(Clone)]
pub struct RedisStore {
    redis_conn: Arc<redis_interface::RedisConnectionPool>,
}

impl std::fmt::Debug for RedisStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheStore")
            .field("redis_conn", &"Redis conn doesn't implement debug")
            .finish()
    }
}

impl RedisStore {
    pub async fn new_without_event_emitter(
        conf: &redis_interface::RedisSettings,
    ) -> error_stack::Result<Self, redis_interface::errors::RedisError> {
        Self::new(conf, Arc::new(NoOpEventEmitter)).await
    }

    pub async fn new(
        conf: &redis_interface::RedisSettings,
        event_emitter: Arc<dyn ExternalServiceEventEmitter>,
    ) -> error_stack::Result<Self, redis_interface::errors::RedisError> {
        Ok(Self {
            redis_conn: Arc::new(
                redis_interface::RedisConnectionPool::new(conf, event_emitter).await?,
            ),
        })
    }

    pub fn set_error_callback(&self, callback: tokio::sync::oneshot::Sender<()>) {
        let redis_clone = self.redis_conn.clone();
        let _task_handle = tokio::spawn(
            async move {
                redis_clone.on_error(callback).await;
            }
            .in_current_span(),
        );
    }

    pub fn get_redis_pool(
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

    pub fn clone_pool_with_prefix(&self, key_prefix: &str) -> Self {
        Self {
            redis_conn: Arc::new(redis_interface::RedisConnectionPool::clone(
                &self.redis_conn,
                key_prefix,
            )),
        }
    }
}
