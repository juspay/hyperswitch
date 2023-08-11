use std::sync::Arc;

use error_stack::ResultExt;
use masking::StrongSecret;
use redis::{kv_store::RedisConnInterface, RedisStore};
pub mod config;
pub mod database;
pub mod payments;
pub mod redis;
pub mod refund;

use database::store::PgPool;
use redis_interface::errors::RedisError;

pub use crate::database::store::DatabaseStore;

#[derive(Debug, Clone)]
pub struct RouterStore<T: DatabaseStore> {
    db_store: T,
    cache_store: RedisStore,
    master_encryption_key: StrongSecret<Vec<u8>>,
}

#[async_trait::async_trait]
impl<T: DatabaseStore> DatabaseStore for RouterStore<T>
where
    T::Config: Send,
{
    type Config = (
        T::Config,
        redis_interface::RedisSettings,
        StrongSecret<Vec<u8>>,
        tokio::sync::oneshot::Sender<()>,
        &'static str,
    );
    async fn new(config: Self::Config, test_transaction: bool) -> Self {
        let (db_conf, cache_conf, encryption_key, cache_error_signal, inmemory_cache_stream) =
            config;
        if test_transaction {
            Self::test_store(db_conf, &cache_conf, encryption_key).await
        } else {
            Self::from_config(
                db_conf,
                &cache_conf,
                encryption_key,
                cache_error_signal,
                inmemory_cache_stream,
            )
            .await
        }
    }
    fn get_master_pool(&self) -> &PgPool {
        self.db_store.get_master_pool()
    }
    fn get_replica_pool(&self) -> &PgPool {
        self.db_store.get_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for RouterStore<T> {
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError> {
        self.cache_store.get_redis_conn()
    }
}

impl<T: DatabaseStore> RouterStore<T> {
    pub async fn from_config(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: StrongSecret<Vec<u8>>,
        cache_error_signal: tokio::sync::oneshot::Sender<()>,
        inmemory_cache_stream: &str,
    ) -> Self {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, false).await;
        #[allow(clippy::expect_used)]
        let cache_store = RedisStore::new(cache_conf)
            .await
            .expect("Failed to create cache store");
        cache_store.set_error_callback(cache_error_signal);
        #[allow(clippy::expect_used)]
        cache_store
            .subscribe_to_channel(inmemory_cache_stream)
            .await
            .expect("Failed to subscribe to inmemory cache stream");
        Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
        }
    }

    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        &self.master_encryption_key
    }

    pub async fn test_store(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: StrongSecret<Vec<u8>>,
    ) -> Self {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, true).await;
        #[allow(clippy::expect_used)]
        let cache_store = RedisStore::new(cache_conf)
            .await
            .expect("Failed to create cache store");
        Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KVRouterStore<T: DatabaseStore> {
    router_store: RouterStore<T>,
    drainer_stream_name: String,
    drainer_num_partitions: u8,
}

#[async_trait::async_trait]
impl<T> DatabaseStore for KVRouterStore<T>
where
    RouterStore<T>: DatabaseStore,
    T: DatabaseStore,
{
    type Config = (RouterStore<T>, String, u8);
    async fn new(config: Self::Config, _test_transaction: bool) -> Self {
        let (router_store, drainer_stream_name, drainer_num_partitions) = config;
        Self::from_store(router_store, drainer_stream_name, drainer_num_partitions)
    }
    fn get_master_pool(&self) -> &PgPool {
        self.router_store.get_master_pool()
    }
    fn get_replica_pool(&self) -> &PgPool {
        self.router_store.get_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for KVRouterStore<T> {
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError> {
        self.router_store.get_redis_conn()
    }
}
impl<T: DatabaseStore> KVRouterStore<T> {
    pub fn from_store(
        store: RouterStore<T>,
        drainer_stream_name: String,
        drainer_num_partitions: u8,
    ) -> Self {
        Self {
            router_store: store,
            drainer_stream_name,
            drainer_num_partitions,
        }
    }

    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        self.router_store.master_key()
    }

    pub fn get_drainer_stream_name(&self, shard_key: &str) -> String {
        format!("{{{}}}_{}", shard_key, self.drainer_stream_name)
    }

    pub async fn push_to_drainer_stream<R>(
        &self,
        redis_entry: diesel_models::kv::TypedSql,
        partition_key: redis::kv_store::PartitionKey<'_>,
    ) -> error_stack::Result<(), RedisError>
    where
        R: crate::redis::kv_store::KvStorePartition,
    {
        let shard_key = R::shard_key(partition_key, self.drainer_num_partitions);
        let stream_name = self.get_drainer_stream_name(&shard_key);
        self.router_store
            .cache_store
            .redis_conn
            .stream_append_entry(
                &stream_name,
                &redis_interface::RedisEntryId::AutoGeneratedID,
                redis_entry
                    .to_field_value_pairs()
                    .change_context(RedisError::JsonSerializationFailed)?,
            )
            .await
            .change_context(RedisError::StreamAppendFailed)
    }
}
