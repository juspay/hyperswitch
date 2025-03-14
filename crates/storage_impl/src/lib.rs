use std::{fmt::Debug, sync::Arc};

use diesel_models as store;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    errors::{StorageError, StorageResult},
    merchant_key_store::MerchantKeyStore,
};
use masking::StrongSecret;
use redis::{kv_store::RedisConnInterface, pub_sub::PubSubInterface, RedisStore};
mod address;
pub mod callback_mapper;
pub mod config;
pub mod connection;
pub mod customers;
pub mod database;
pub mod errors;
pub mod kv_router_store;
pub mod lookup;
pub mod mandate;
pub mod metrics;
pub mod mock_db;
pub mod payment_method;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod redis;
pub mod refund;
mod reverse_lookup;
pub mod utils;

use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use database::store::PgPool;
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};
pub use mock_db::MockDb;
use redis_interface::{errors::RedisError, RedisConnectionPool, SaddReply};

pub use crate::database::store::DatabaseStore;
#[cfg(not(feature = "payouts"))]
pub use crate::database::store::Store;

#[derive(Debug, Clone)]
pub struct RouterStore<T: DatabaseStore> {
    db_store: T,
    cache_store: Arc<RedisStore>,
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
    async fn new(
        config: Self::Config,
        tenant_config: &dyn config::TenantConfig,
        test_transaction: bool,
    ) -> StorageResult<Self> {
        let (db_conf, cache_conf, encryption_key, cache_error_signal, inmemory_cache_stream) =
            config;
        if test_transaction {
            Self::test_store(db_conf, tenant_config, &cache_conf, encryption_key)
                .await
                .attach_printable("failed to create test router store")
        } else {
            Self::from_config(
                db_conf,
                tenant_config,
                encryption_key,
                Self::cache_store(&cache_conf, cache_error_signal).await?,
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

    fn get_accounts_master_pool(&self) -> &PgPool {
        self.db_store.get_accounts_master_pool()
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        self.db_store.get_accounts_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for RouterStore<T> {
    fn get_redis_conn(&self) -> error_stack::Result<Arc<RedisConnectionPool>, RedisError> {
        self.cache_store.get_redis_conn()
    }
}

impl<T: DatabaseStore> RouterStore<T> {
    pub async fn from_config(
        db_conf: T::Config,
        tenant_config: &dyn config::TenantConfig,
        encryption_key: StrongSecret<Vec<u8>>,
        cache_store: Arc<RedisStore>,
        inmemory_cache_stream: &str,
    ) -> StorageResult<Self> {
        let db_store = T::new(db_conf, tenant_config, false).await?;
        let redis_conn = cache_store.redis_conn.clone();
        let cache_store = Arc::new(RedisStore {
            redis_conn: Arc::new(RedisConnectionPool::clone(
                &redis_conn,
                tenant_config.get_redis_key_prefix(),
            )),
        });
        cache_store
            .redis_conn
            .subscribe(inmemory_cache_stream)
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

    pub async fn cache_store(
        cache_conf: &redis_interface::RedisSettings,
        cache_error_signal: tokio::sync::oneshot::Sender<()>,
    ) -> StorageResult<Arc<RedisStore>> {
        let cache_store = RedisStore::new(cache_conf)
            .await
            .change_context(StorageError::InitializationError)
            .attach_printable("Failed to create cache store")?;
        cache_store.set_error_callback(cache_error_signal);
        Ok(Arc::new(cache_store))
    }

    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        &self.master_encryption_key
    }

    pub async fn call_database<D, R, M>(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        execute_query: R,
    ) -> error_stack::Result<D, StorageError>
    where
        D: Debug + Sync + Conversion,
        R: futures::Future<Output = error_stack::Result<M, diesel_models::errors::DatabaseError>>
            + Send,
        M: ReverseConversion<D>,
    {
        execute_query
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })?
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    pub async fn find_resources<D, R, M>(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        execute_query: R,
    ) -> error_stack::Result<Vec<D>, StorageError>
    where
        D: Debug + Sync + Conversion,
        R: futures::Future<
                Output = error_stack::Result<Vec<M>, diesel_models::errors::DatabaseError>,
            > + Send,
        M: ReverseConversion<D>,
    {
        let resource_futures = execute_query
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })?
            .into_iter()
            .map(|resource| async {
                resource
                    .convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .collect::<Vec<_>>();

        let resources = futures::future::try_join_all(resource_futures).await?;

        Ok(resources)
    }

    /// # Panics
    ///
    /// Will panic if `CONNECTOR_AUTH_FILE_PATH` is not set
    pub async fn test_store(
        db_conf: T::Config,
        tenant_config: &dyn config::TenantConfig,
        cache_conf: &redis_interface::RedisSettings,
        encryption_key: StrongSecret<Vec<u8>>,
    ) -> StorageResult<Self> {
        // TODO: create an error enum and return proper error here
        let db_store = T::new(db_conf, tenant_config, true).await?;
        let cache_store = RedisStore::new(cache_conf)
            .await
            .change_context(StorageError::InitializationError)
            .attach_printable("failed to create redis cache")?;
        Ok(Self {
            db_store,
            cache_store: Arc::new(cache_store),
            master_encryption_key: encryption_key,
            request_id: None,
        })
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
    diesel_error: diesel_models::errors::DatabaseError,
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
        _ => StorageError::DatabaseError(error_stack::report!(diesel_error)),
    }
}

#[async_trait::async_trait]
pub trait UniqueConstraints {
    fn unique_constraints(&self) -> Vec<String>;
    fn table_name(&self) -> &str;
    async fn check_for_constraints(
        &self,
        redis_conn: &Arc<RedisConnectionPool>,
    ) -> CustomResult<(), RedisError> {
        let constraints = self.unique_constraints();
        let sadd_result = redis_conn
            .sadd(
                &format!("unique_constraint:{}", self.table_name()).into(),
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

#[cfg(feature = "v2")]
impl UniqueConstraints for diesel_models::PaymentIntent {
    fn unique_constraints(&self) -> Vec<String> {
        vec![self.id.get_string_repr().to_owned()]
    }

    fn table_name(&self) -> &str {
        "PaymentIntent"
    }
}

#[cfg(feature = "v1")]
impl UniqueConstraints for diesel_models::PaymentIntent {
    #[cfg(feature = "v1")]
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "pi_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr()
        )]
    }

    #[cfg(feature = "v2")]
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("pi_{}", self.id.get_string_repr())]
    }

    fn table_name(&self) -> &str {
        "PaymentIntent"
    }
}

#[cfg(feature = "v1")]
impl UniqueConstraints for diesel_models::PaymentAttempt {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "pa_{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id
        )]
    }
    fn table_name(&self) -> &str {
        "PaymentAttempt"
    }
}

impl UniqueConstraints for diesel_models::Refund {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "refund_{}_{}",
            self.merchant_id.get_string_repr(),
            self.refund_id
        )]
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

#[cfg(feature = "payouts")]
impl UniqueConstraints for diesel_models::Payouts {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "po_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payout_id
        )]
    }
    fn table_name(&self) -> &str {
        "Payouts"
    }
}

#[cfg(feature = "payouts")]
impl UniqueConstraints for diesel_models::PayoutAttempt {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "poa_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payout_attempt_id
        )]
    }
    fn table_name(&self) -> &str {
        "PayoutAttempt"
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl UniqueConstraints for diesel_models::PaymentMethod {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("paymentmethod_{}", self.payment_method_id)]
    }
    fn table_name(&self) -> &str {
        "PaymentMethod"
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl UniqueConstraints for diesel_models::PaymentMethod {
    fn unique_constraints(&self) -> Vec<String> {
        vec![self.id.get_string_repr().to_owned()]
    }
    fn table_name(&self) -> &str {
        "PaymentMethod"
    }
}

impl UniqueConstraints for diesel_models::Mandate {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "mand_{}_{}",
            self.merchant_id.get_string_repr(),
            self.mandate_id
        )]
    }
    fn table_name(&self) -> &str {
        "Mandate"
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl UniqueConstraints for diesel_models::Customer {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!(
            "customer_{}_{}",
            self.customer_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
        )]
    }
    fn table_name(&self) -> &str {
        "Customer"
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl UniqueConstraints for diesel_models::Customer {
    fn unique_constraints(&self) -> Vec<String> {
        vec![format!("customer_{}", self.id.get_string_repr())]
    }
    fn table_name(&self) -> &str {
        "Customer"
    }
}

#[cfg(not(feature = "payouts"))]
impl<T: DatabaseStore> PayoutAttemptInterface for RouterStore<T> {}
#[cfg(not(feature = "payouts"))]
impl<T: DatabaseStore> PayoutsInterface for RouterStore<T> {}
