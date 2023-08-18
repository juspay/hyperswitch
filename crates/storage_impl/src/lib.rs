use std::sync::Arc;

use common_utils::errors::CustomResult;
use diesel_models::{self as store};
use error_stack::ResultExt;
use external_services::kms::{decrypt::KmsDecrypt, KmsClient, KmsError};
use futures::lock::Mutex;
use masking::{Deserialize, ExposeInterface, StrongSecret};
use redis::{kv_store::RedisConnInterface, RedisStore};
pub mod config;
pub mod connection;
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

#[derive(Clone)]
pub struct MockDb {
    pub addresses: Arc<Mutex<Vec<store::Address>>>,
    pub configs: Arc<Mutex<Vec<store::Config>>>,
    pub merchant_accounts: Arc<Mutex<Vec<store::MerchantAccount>>>,
    pub merchant_connector_accounts: Arc<Mutex<Vec<store::MerchantConnectorAccount>>>,
    pub payment_attempts: Arc<Mutex<Vec<store::PaymentAttempt>>>,
    pub payment_intents: Arc<Mutex<Vec<store::PaymentIntent>>>,
    pub payment_methods: Arc<Mutex<Vec<store::PaymentMethod>>>,
    pub customers: Arc<Mutex<Vec<store::Customer>>>,
    pub refunds: Arc<Mutex<Vec<store::Refund>>>,
    pub processes: Arc<Mutex<Vec<store::ProcessTracker>>>,
    pub connector_response: Arc<Mutex<Vec<store::ConnectorResponse>>>,
    pub redis: Arc<redis_interface::RedisConnectionPool>,
    pub api_keys: Arc<Mutex<Vec<store::ApiKey>>>,
    pub ephemeral_keys: Arc<Mutex<Vec<store::EphemeralKey>>>,
    pub cards_info: Arc<Mutex<Vec<store::CardInfo>>>,
    pub events: Arc<Mutex<Vec<store::Event>>>,
    pub disputes: Arc<Mutex<Vec<store::Dispute>>>,
    pub lockers: Arc<Mutex<Vec<store::LockerMockUp>>>,
    pub mandates: Arc<Mutex<Vec<store::Mandate>>>,
    pub captures: Arc<Mutex<Vec<crate::store::capture::Capture>>>,
    pub merchant_key_store: Arc<Mutex<Vec<crate::store::merchant_key_store::MerchantKeyStore>>>,
}

impl MockDb {
    pub async fn new(redis: redis_interface::RedisSettings) -> Self {
        Self {
            addresses: Default::default(),
            configs: Default::default(),
            merchant_accounts: Default::default(),
            merchant_connector_accounts: Default::default(),
            payment_attempts: Default::default(),
            payment_intents: Default::default(),
            payment_methods: Default::default(),
            customers: Default::default(),
            refunds: Default::default(),
            processes: Default::default(),
            connector_response: Default::default(),
            redis: Arc::new(crate::connection::redis_connection(&redis).await),
            api_keys: Default::default(),
            ephemeral_keys: Default::default(),
            cards_info: Default::default(),
            events: Default::default(),
            disputes: Default::default(),
            lockers: Default::default(),
            mandates: Default::default(),
            captures: Default::default(),
            merchant_key_store: Default::default(),
        }
    }
}

impl RedisConnInterface for MockDb {
    fn get_redis_conn(
        &self,
    ) -> Result<Arc<redis_interface::RedisConnectionPool>, error_stack::Report<RedisError>> {
        Ok(self.redis.clone())
    }
}

#[cfg(feature = "kms")]
/// Store the decrypted kms secret values for active use in the application
/// Currently using `StrongSecret` won't have any effect as this struct have smart pointers to heap
/// allocations.
/// note: we can consider adding such behaviour in the future with custom implementation
#[derive(Clone)]
pub struct ActiveKmsSecrets {
    pub jwekey: masking::Secret<Jwekey>,
}
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Jwekey {
    pub locker_key_identifier1: String,
    pub locker_key_identifier2: String,
    pub locker_encryption_key1: String,
    pub locker_encryption_key2: String,
    pub locker_decryption_key1: String,
    pub locker_decryption_key2: String,
    pub vault_encryption_key: String,
    pub vault_private_key: String,
    pub tunnel_private_key: String,
}

#[async_trait::async_trait]
impl KmsDecrypt for Jwekey {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        (
            self.locker_encryption_key1,
            self.locker_encryption_key2,
            self.locker_decryption_key1,
            self.locker_decryption_key2,
            self.vault_encryption_key,
            self.vault_private_key,
            self.tunnel_private_key,
        ) = tokio::try_join!(
            kms_client.decrypt(self.locker_encryption_key1),
            kms_client.decrypt(self.locker_encryption_key2),
            kms_client.decrypt(self.locker_decryption_key1),
            kms_client.decrypt(self.locker_decryption_key2),
            kms_client.decrypt(self.vault_encryption_key),
            kms_client.decrypt(self.vault_private_key),
            kms_client.decrypt(self.tunnel_private_key),
        )?;
        Ok(self)
    }
}

#[cfg(feature = "kms")]
#[async_trait::async_trait]
impl KmsDecrypt for ActiveKmsSecrets {
    type Output = Self;
    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        self.jwekey = self.jwekey.expose().decrypt_inner(kms_client).await?.into();
        Ok(self)
    }
}
