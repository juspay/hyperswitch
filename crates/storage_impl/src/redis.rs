pub mod kv_store;

use std::sync::{atomic, Arc};

use error_stack::{IntoReport, ResultExt};
use redis_interface::PubsubInterface;

use self::kv_store::RedisConnInterface;

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
        tokio::spawn(async move {
            redis_clone.on_error(callback).await;
        });
    }

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

        // TODO: Handle on message failures
        // let redis_clone = self.redis_conn.clone();
        // tokio::spawn(async move {
        // if let Err(e) = redis_clone.on_message().await {
        //     logger::error!(pubsub_err=?e);
        // }
        // });
        Ok(())
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
