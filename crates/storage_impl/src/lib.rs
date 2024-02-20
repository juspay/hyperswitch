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

use common_utils::errors::CustomResult;
use database::store::PgPool;
pub use mock_db::MockDb;
use redis_interface::{errors::RedisError, SaddReply};

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
    async fn new(config: Self::Config, _test_transaction: bool) -> StorageResult<Self> {
        let (router_store, drainer_stream_name, drainer_num_partitions, ttl_for_kv) = config;
        Ok(Self::from_store(
            router_store,
            drainer_stream_name,
            drainer_num_partitions,
            ttl_for_kv,
        ))
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

#[async_trait::async_trait]
pub trait UniqueConstraints {
    fn unique_constraints(&self) -> Vec<String>;
    fn table_name(&self) -> &str;
    async fn check_for_constraints(
        &self,
        redis_conn: &Arc<redis_interface::RedisConnectionPool>,
    ) -> CustomResult<(), RedisError> {
        let constraints = self.unique_constraints();
        let sadd_result = redis_conn
            .sadd(
                &format!("unique_constraint_{}", self.table_name()),
                constraints,
            )
            .await?;

        match sadd_result {
            SaddReply::KeyNotSet => Err(error_stack::report!(RedisError::SetAddMembersFailed)),
            SaddReply::KeySet => Ok(()),
        }
    }
}

impl UniqueConstraints for diesel_models::Address {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("address_{}", self.address_id)]
    }
    fn table_name(&self) -> &str {
        "Address"
    }
}

impl UniqueConstraints for diesel_models::PaymentIntent {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("pi_{}_{}", self.merchant_id, self.payment_id)]
    }
    fn table_name(&self) -> &str {
        "PaymentIntent"
    }
}

impl UniqueConstraints for diesel_models::PaymentAttempt {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "pa_{}_{}_{}",
            self.merchant_id, self.payment_id, self.attempt_id
        )]
    }
    fn table_name(&self) -> &str {
        "PaymentAttempt"
    }
}

impl UniqueConstraints for diesel_models::Refund {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("refund_{}_{}", self.merchant_id, self.refund_id)]
    }
    fn table_name(&self) -> &str {
        "Refund"
    }
}

impl UniqueConstraints for diesel_models::ReverseLookup {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("reverselookup_{}", self.lookup_id)]
    }
    fn table_name(&self) -> &str {
        "ReverseLookup"
    }
}
