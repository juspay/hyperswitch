use std::sync::Arc;

use data_models::errors::{StorageError, StorageResult};
use diesel_models as store;
use error_stack::ResultExt;
use masking::StrongSecret;
use redis::{kv_store::RedisConnInterface, RedisStore};
mod address;
pub mod config;
pub mod connection;
pub mod database;
pub mod errors;
mod lookup;
pub mod metrics;
pub mod mock_db;
pub mod payments;
pub mod redis;
pub mod refund;
mod reverse_lookup;
mod utils;

use database::store::PgPool;
pub use mock_db::MockDb;
use redis_interface::errors::RedisError;

pub use crate::database::store::DatabaseStore;

#[derive(Debug, Clone)]
pub struct RouterStore<T: DatabaseStore> {
    db_store: T,
    cache_store: RedisStore,
    master_encryption_key: StrongSecret<Vec<u8>>,
    pub request_id: Option<String>,
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
        /// Asynchronously creates a new instance of Storage, either for testing purposes or for regular use.
    /// 
    /// # Arguments
    /// 
    /// * `config` - A tuple containing the database configuration, cache configuration, encryption key, cache error signal, and in-memory cache stream.
    /// * `test_transaction` - A boolean indicating whether the storage instance is for testing purposes.
    ///
    /// # Returns
    /// 
    /// A `StorageResult` containing the newly created `Storage` instance, or an error if the creation fails.
    ///
    async fn new(config: Self::Config, test_transaction: bool) -> StorageResult<Self> {
        let (db_conf, cache_conf, encryption_key, cache_error_signal, inmemory_cache_stream) =
            config;
        if test_transaction {
            Self::test_store(db_conf, &cache_conf, encryption_key)
                .await
                .attach_printable("failed to create test router store")
        } else {
            Self::from_config(
                db_conf,
                &cache_conf,
                encryption_key,
                cache_error_signal,
                inmemory_cache_stream,
            )
            .await
            .attach_printable("failed to create store")
        }
    }
        /// Retrieves the master connection pool for the database store.
    fn get_master_pool(&self) -> &PgPool {
        self.db_store.get_master_pool()
    }
        /// Retrieves the replica pool from the database store.
    fn get_replica_pool(&self) -> &PgPool {
        self.db_store.get_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for RouterStore<T> {
        /// Retrieves a Redis connection from the cache store and returns it as a thread-safe reference-counted Arc wrapped in a Result.
    /// 
    /// # Errors
    /// Returns a RedisError if there was an issue retrieving the Redis connection from the cache store.
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError> {
        self.cache_store.get_redis_conn()
    }
}

impl<T: DatabaseStore> RouterStore<T> {
        /// Creates a new instance of Storage using the provided database configuration, cache configuration, encryption key, error signal for cache, and in-memory cache stream. It initializes the database store and cache store, sets the error callback for cache store, subscribes to the specified in-memory cache stream, and returns a result containing the initialized Storage instance.
    pub async fn from_config(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: StrongSecret<Vec<u8>>,
        cache_error_signal: tokio::sync::oneshot::Sender<()>,
        inmemory_cache_stream: &str,
    ) -> StorageResult<Self> {
        let db_store = T::new(db_conf, false).await?;
        let cache_store = RedisStore::new(cache_conf)
            .await
            .change_context(StorageError::InitializationError)
            .attach_printable("Failed to create cache store")?;
        cache_store.set_error_callback(cache_error_signal);
        cache_store
            .subscribe_to_channel(inmemory_cache_stream)
            .await
            .change_context(StorageError::InitializationError)
            .attach_printable("Failed to subscribe to inmemory cache stream")?;
        Ok(Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
            request_id: None,
        })
    }

        /// Returns a reference to the master encryption key.
    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        &self.master_encryption_key
    }

    /// # Panics
    ///
    /// Will panic if `CONNECTOR_AUTH_FILE_PATH` is not set
    pub async fn test_store(
        db_conf: T::Config,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: StrongSecret<Vec<u8>>,
    ) -> StorageResult<Self> {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, true).await?;
        let cache_store = RedisStore::new(cache_conf)
            .await
            .change_context(StorageError::InitializationError)
            .attach_printable("failed to create redis cache")?;
        Ok(Self {
            db_store,
            cache_store,
            master_encryption_key: encryption_key,
            request_id: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct KVRouterStore<T: DatabaseStore> {
    router_store: RouterStore<T>,
    drainer_stream_name: String,
    drainer_num_partitions: u8,
    ttl_for_kv: u32,
    pub request_id: Option<String>,
}

#[async_trait::async_trait]
impl<T> DatabaseStore for KVRouterStore<T>
where
    RouterStore<T>: DatabaseStore,
    T: DatabaseStore,
{
    type Config = (RouterStore<T>, String, u8, u32);
        /// Creates a new instance of Storage using the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - A tuple containing the router store, drainer stream name, drainer number of partitions, and time-to-live for key-value pairs.
    /// * `_test_transaction` - A boolean indicating whether to use test transactions.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the newly created instance of `Storage`.
    ///
    async fn new(config: Self::Config, _test_transaction: bool) -> StorageResult<Self> {
        let (router_store, drainer_stream_name, drainer_num_partitions, ttl_for_kv) = config;
        Ok(Self::from_store(
            router_store,
            drainer_stream_name,
            drainer_num_partitions,
            ttl_for_kv,
        ))
    }
        /// This method returns a reference to the master database connection pool.
    fn get_master_pool(&self) -> &PgPool {
        self.router_store.get_master_pool()
    }
        /// Retrieves the replica pool from the router store.
    fn get_replica_pool(&self) -> &PgPool {
        self.router_store.get_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for KVRouterStore<T> {
        /// Retrieves a Redis connection from the router store and returns it as an Arc-wrapped RedisConnectionPool.
    /// 
    /// # Returns
    /// 
    /// Returns a Result containing the Arc<RedisConnectionPool> if successful, otherwise returns a RedisError.
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError> {
        self.router_store.get_redis_conn()
    }
}

impl<T: DatabaseStore> KVRouterStore<T> {
        /// Create a new instance of Self using the provided RouterStore, drainer stream name, drainer number of partitions, and time-to-live for key-value pairs.
    pub fn from_store(
        store: RouterStore<T>,
        drainer_stream_name: String,
        drainer_num_partitions: u8,
        ttl_for_kv: u32,
    ) -> Self {
        let request_id = store.request_id.clone();

        Self {
            router_store: store,
            drainer_stream_name,
            drainer_num_partitions,
            ttl_for_kv,
            request_id,
        }
    }

        /// Returns a reference to the master key stored in the router store.
    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        self.router_store.master_key()
    }

        /// Returns the drainer stream name for the given shard key by formatting it as "{shard_key}_{drainer_stream_name}".
    pub fn get_drainer_stream_name(&self, shard_key: &str) -> String {
        format!("{{{}}}_{}", shard_key, self.drainer_stream_name)
    }

        /// Asynchronously pushes the given `redis_entry` to the drainer stream using the specified `partition_key`. The method retrieves the global ID and request ID, calculates the shard key, and determines the stream name based on the shard key. It then appends the redis entry to the stream, increments the KV_PUSHED_TO_DRAINER metric if successful, or increments the KV_FAILED_TO_PUSH_TO_DRAINER metric and returns an error if the operation fails.
    pub async fn push_to_drainer_stream<R>(
        &self,
        redis_entry: diesel_models::kv::TypedSql,
        partition_key: redis::kv_store::PartitionKey<'_>,
    ) -> error_stack::Result<(), RedisError>
    where
        R: crate::redis::kv_store::KvStorePartition,
    {
        let global_id = format!("{}", partition_key);
        let request_id = self.request_id.clone().unwrap_or_default();

        let shard_key = R::shard_key(partition_key, self.drainer_num_partitions);
        let stream_name = self.get_drainer_stream_name(&shard_key);
        self.router_store
            .cache_store
            .redis_conn
            .stream_append_entry(
                &stream_name,
                &redis_interface::RedisEntryId::AutoGeneratedID,
                redis_entry
                    .to_field_value_pairs(request_id, global_id)
                    .change_context(RedisError::JsonSerializationFailed)?,
            )
            .await
            .map(|_| metrics::KV_PUSHED_TO_DRAINER.add(&metrics::CONTEXT, 1, &[]))
            .map_err(|err| {
                metrics::KV_FAILED_TO_PUSH_TO_DRAINER.add(&metrics::CONTEXT, 1, &[]);
                err
            })
            .change_context(RedisError::StreamAppendFailed)
    }
}

// TODO: This should not be used beyond this crate
// Remove the pub modified once StorageScheme usage is completed
pub trait DataModelExt {
    type StorageModel;
    fn to_storage_model(self) -> Self::StorageModel;
    fn from_storage_model(storage_model: Self::StorageModel) -> Self;
}

/// Converts a diesel database error to a custom StorageError enum.
pub(crate) fn diesel_error_to_data_error(
    diesel_error: &diesel_models::errors::DatabaseError,
) -> StorageError {
    match diesel_error {
        diesel_models::errors::DatabaseError::DatabaseConnectionError => {
            StorageError::DatabaseConnectionError
        }
        diesel_models::errors::DatabaseError::NotFound => {
            StorageError::ValueNotFound("Value not found".to_string())
        }
        diesel_models::errors::DatabaseError::UniqueViolation => StorageError::DuplicateValue {
            entity: "entity ",
            key: None,
        },
        _ => StorageError::DatabaseError(error_stack::report!(*diesel_error)),
    }
}
