pub mod api;
pub mod authentication;
pub mod encryption;
pub mod logger;

use std::sync::{atomic, Arc};

use error_stack::{IntoReport, ResultExt};
use futures::StreamExt;
use redis_interface::{errors as redis_errors, PubsubInterface};

pub use self::{api::*, encryption::*};
use crate::{
    async_spawn,
    cache::CONFIG_CACHE,
    connection::{diesel_make_pg_pool, PgPool},
    consts,
    core::errors,
};

#[async_trait::async_trait]
pub trait PubSubInterface {
    async fn subscribe(
        &self,
        channel: &str,
    ) -> errors::CustomResult<usize, redis_errors::RedisError>;
    async fn publish(
        &self,
        channel: &str,
        key: &str,
    ) -> errors::CustomResult<usize, redis_errors::RedisError>;
    async fn on_message(&self) -> errors::CustomResult<(), redis_errors::RedisError>;
}

#[async_trait::async_trait]
impl PubSubInterface for redis_interface::RedisConnectionPool {
    #[inline]
    async fn subscribe(
        &self,
        channel: &str,
    ) -> errors::CustomResult<usize, redis_errors::RedisError> {
        self.subscriber
            .subscribe(channel)
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }
    #[inline]
    async fn publish(
        &self,
        channel: &str,
        key: &str,
    ) -> errors::CustomResult<usize, redis_errors::RedisError> {
        self.publisher
            .publish(channel, key)
            .await
            .into_report()
            .change_context(redis_errors::RedisError::SubscribeError)
    }
    #[inline]
    async fn on_message(&self) -> errors::CustomResult<(), redis_errors::RedisError> {
        let mut message = self.subscriber.on_message();
        while let Some((_, key)) = message.next().await {
            let key = key
                .as_string()
                .ok_or::<redis_errors::RedisError>(redis_errors::RedisError::DeleteFailed)?;

            self.delete_key(&key).await?;
            CONFIG_CACHE.invalidate(&key).await;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Store {
    pub master_pool: PgPool,
    #[cfg(feature = "olap")]
    pub replica_pool: PgPool,
    pub redis_conn: Arc<redis_interface::RedisConnectionPool>,
    #[cfg(feature = "kv_store")]
    pub(crate) config: StoreConfig,
}

#[cfg(feature = "kv_store")]
#[derive(Clone)]
pub(crate) struct StoreConfig {
    pub(crate) drainer_stream_name: String,
    pub(crate) drainer_num_partitions: u8,
}

impl Store {
    pub async fn new(config: &crate::configs::settings::Settings, test_transaction: bool) -> Self {
        let redis_conn = Arc::new(crate::connection::redis_connection(config).await);
        let redis_clone = redis_conn.clone();

        let subscriber_conn = redis_conn.clone();

        redis_conn.subscribe(consts::PUB_SUB_CHANNEL).await.ok();

        async_spawn!({
            if let Err(e) = subscriber_conn.on_message().await {
                logger::error!(pubsub_err=?e);
            }
        });

        async_spawn!({
            redis_clone.on_error().await;
        });

        Self {
            master_pool: diesel_make_pg_pool(
                &config.master_database,
                test_transaction,
                #[cfg(feature = "kms")]
                &config.kms,
            )
            .await,
            #[cfg(feature = "olap")]
            replica_pool: diesel_make_pg_pool(
                &config.replica_database,
                test_transaction,
                #[cfg(feature = "kms")]
                &config.kms,
            )
            .await,
            redis_conn,
            #[cfg(feature = "kv_store")]
            config: StoreConfig {
                drainer_stream_name: config.drainer.stream_name.clone(),
                drainer_num_partitions: config.drainer.num_partitions,
            },
        }
    }

    #[cfg(feature = "kv_store")]
    pub fn get_drainer_stream_name(&self, shard_key: &str) -> String {
        // Example: {shard_5}_drainer_stream
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }

    pub fn redis_conn(
        &self,
    ) -> errors::CustomResult<Arc<redis_interface::RedisConnectionPool>, redis_errors::RedisError>
    {
        if self
            .redis_conn
            .is_redis_available
            .load(atomic::Ordering::SeqCst)
        {
            Ok(self.redis_conn.clone())
        } else {
            Err(redis_errors::RedisError::RedisConnectionError.into())
        }
    }

    #[cfg(feature = "kv_store")]
    pub(crate) async fn push_to_drainer_stream<T>(
        &self,
        redis_entry: storage_models::kv::TypedSql,
        partition_key: crate::utils::storage_partitioning::PartitionKey<'_>,
    ) -> crate::core::errors::CustomResult<(), crate::core::errors::StorageError>
    where
        T: crate::utils::storage_partitioning::KvStorePartition,
    {
        let shard_key = T::shard_key(partition_key, self.config.drainer_num_partitions);
        let stream_name = self.get_drainer_stream_name(&shard_key);
        self.redis_conn
            .stream_append_entry(
                &stream_name,
                &redis_interface::RedisEntryId::AutoGeneratedID,
                redis_entry
                    .to_field_value_pairs()
                    .change_context(crate::core::errors::StorageError::KVError)?,
            )
            .await
            .change_context(crate::core::errors::StorageError::KVError)
    }
}
