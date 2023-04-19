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
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
pub mod process_tracker;
pub mod queue;
pub mod refund;
pub mod reverse_lookup;

use std::sync::Arc;

use futures::lock::Mutex;

use crate::{services::Store, types::storage};

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
    + process_tracker::ProcessTrackerInterface
    + queue::QueueInterface
    + refund::RefundInterface
    + reverse_lookup::ReverseLookupInterface
    + cards_info::CardsInfoInterface
    + 'static
{
    async fn close(&mut self) {}
}

#[async_trait::async_trait]
impl StorageInterface for Store {
    #[allow(clippy::expect_used)]
    async fn close(&mut self) {
        std::sync::Arc::get_mut(&mut self.redis_conn)
            .expect("Redis connection pool cannot be closed")
            .close_connections()
            .await;
    }
}

#[derive(Clone)]
pub struct MockDb {
    merchant_accounts: Arc<Mutex<Vec<storage::MerchantAccount>>>,
    merchant_connector_accounts: Arc<Mutex<Vec<storage::MerchantConnectorAccount>>>,
    payment_attempts: Arc<Mutex<Vec<storage::PaymentAttempt>>>,
    payment_intents: Arc<Mutex<Vec<storage::PaymentIntent>>>,
    customers: Arc<Mutex<Vec<storage::Customer>>>,
    refunds: Arc<Mutex<Vec<storage::Refund>>>,
    processes: Arc<Mutex<Vec<storage::ProcessTracker>>>,
    connector_response: Arc<Mutex<Vec<storage::ConnectorResponse>>>,
    redis: Arc<redis_interface::RedisConnectionPool>,
}

impl MockDb {
    pub async fn new(redis: &crate::configs::settings::Settings) -> Self {
        Self {
            merchant_accounts: Default::default(),
            merchant_connector_accounts: Default::default(),
            payment_attempts: Default::default(),
            payment_intents: Default::default(),
            customers: Default::default(),
            refunds: Default::default(),
            processes: Default::default(),
            connector_response: Default::default(),
            redis: Arc::new(crate::connection::redis_connection(redis).await),
        }
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    #[allow(clippy::expect_used)]
    async fn close(&mut self) {
        std::sync::Arc::get_mut(&mut self.redis)
            .expect("Redis connection pool cannot be closed")
            .close_connections()
            .await;
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &str,
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

dyn_clone::clone_trait_object!(StorageInterface);
