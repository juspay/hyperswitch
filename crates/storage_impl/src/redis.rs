pub mod cache;
pub mod kv_store;
pub mod pub_sub;

use std::sync::{atomic, Arc};

use router_env::tracing::Instrument;

use self::{kv_store::RedisConnInterface, pub_sub::PubSubInterface};

#[derive(Clone)]
pub struct RedisStore {
    // Maybe expose the redis_conn via traits instead of the making the field public
    pub(crate) redis_conn: Arc<redis_interface::RedisConnectionPool>,
}

impl std::fmt::Debug for RedisStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheStore")
            .field("redis_conn", &"Redis conn doesn't implement debug")
            .finish()
    }
}

impl RedisStore {
    pub async fn new(
        conf: &redis_interface::RedisSettings,
    ) -> error_stack::Result<Self, redis_interface::errors::RedisError> {
        Ok(Self {
            redis_conn: Arc::new(redis_interface::RedisConnectionPool::new(conf).await?),
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
}

impl RedisConnInterface for RedisStore {
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
