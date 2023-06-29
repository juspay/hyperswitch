pub mod address;
pub mod api_keys;
pub mod cache;
pub mod cards_info;
pub mod configs;
pub mod connector_response;
pub mod customers;
pub mod dispute;
pub mod ephemeral_key;
pub mod events;
pub mod file;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
pub mod process_tracker;
pub mod queue;
pub mod refund;
pub mod reverse_lookup;

use std::sync::Arc;

use storage_models::services::{Store, MockDb};

use crate::{
    services,
};

#[derive(PartialEq, Eq)]
pub enum StorageImpl {
    Postgresql,
    PostgresqlTest,
    Mock,
}

#[async_trait::async_trait]
pub trait StorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + address::AddressInterface
    + api_keys::ApiKeyInterface
    + configs::ConfigInterface
    + connector_response::ConnectorResponseInterface
    + customers::CustomerInterface
    + dispute::DisputeInterface
    + ephemeral_key::EphemeralKeyInterface
    + events::EventInterface
    + file::FileMetadataInterface
    + locker_mock_up::LockerMockUpInterface
    + mandate::MandateInterface
    + merchant_account::MerchantAccountInterface
    + merchant_connector_account::ConnectorAccessToken
    + merchant_connector_account::MerchantConnectorAccountInterface
    + payment_attempt::PaymentAttemptInterface
    + payment_intent::PaymentIntentInterface
    + payment_method::PaymentMethodInterface
    + scheduler::SchedulerInterface
    + refund::RefundInterface
    + reverse_lookup::ReverseLookupInterface
    + cards_info::CardsInfoInterface
    + merchant_key_store::MerchantKeyStoreInterface
    + MasterKeyInterface
    + services::RedisConnInterface
    + 'static
{
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface>;
}

pub trait MasterKeyInterface {
    fn get_master_key(&self) -> &[u8];
}

impl MasterKeyInterface for Store {
    fn get_master_key(&self) -> &[u8] {
        &self.master_key
    }
}

/// Default dummy key for MockDb
impl MasterKeyInterface for MockDb {
    fn get_master_key(&self) -> &[u8] {
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }
}

#[async_trait::async_trait]
impl StorageInterface for Store {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &'static str,
) -> common_utils::errors::CustomResult<T, redis_interface::errors::RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;
    use error_stack::ResultExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(redis_interface::errors::RedisError::JsonDeserializationFailed)
}

impl services::RedisConnInterface for MockDb {
    fn get_redis_conn(&self) -> Arc<redis_interface::RedisConnectionPool> {
        self.redis.clone()
    }
}

dyn_clone::clone_trait_object!(StorageInterface);
