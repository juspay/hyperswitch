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
pub mod fraud_check;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
pub mod payout_attempt;
pub mod payouts;
pub mod process_tracker;
pub mod queue;
pub mod refund;
pub mod reverse_lookup;

use std::sync::Arc;

use futures::lock::Mutex;

use crate::{
    services::{self, Store},
    types::storage,
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
    + fraud_check::FraudCheckInterface
    + locker_mock_up::LockerMockUpInterface
    + mandate::MandateInterface
    + merchant_account::MerchantAccountInterface
    + merchant_connector_account::ConnectorAccessToken
    + merchant_connector_account::MerchantConnectorAccountInterface
    + payment_attempt::PaymentAttemptInterface
    + payment_intent::PaymentIntentInterface
    + payment_method::PaymentMethodInterface
    + payout_attempt::PayoutAttemptInterface
    + payouts::PayoutsInterface
    + process_tracker::ProcessTrackerInterface
    + queue::QueueInterface
    + refund::RefundInterface
    + reverse_lookup::ReverseLookupInterface
    + cards_info::CardsInfoInterface
    + merchant_key_store::MerchantKeyStoreInterface
    + MasterKeyInterface
    + services::RedisConnInterface
    + 'static
{
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
impl StorageInterface for Store {}

#[derive(Clone)]
pub struct MockDb {
    addresses: Arc<Mutex<Vec<storage::Address>>>,
    configs: Arc<Mutex<Vec<storage::Config>>>,
    merchant_accounts: Arc<Mutex<Vec<storage::MerchantAccount>>>,
    merchant_connector_accounts: Arc<Mutex<Vec<storage::MerchantConnectorAccount>>>,
    payment_attempts: Arc<Mutex<Vec<storage::PaymentAttempt>>>,
    payment_intents: Arc<Mutex<Vec<storage::PaymentIntent>>>,
    payment_methods: Arc<Mutex<Vec<storage::PaymentMethod>>>,
    customers: Arc<Mutex<Vec<storage::Customer>>>,
    refunds: Arc<Mutex<Vec<storage::Refund>>>,
    processes: Arc<Mutex<Vec<storage::ProcessTracker>>>,
    connector_response: Arc<Mutex<Vec<storage::ConnectorResponse>>>,
    redis: Arc<redis_interface::RedisConnectionPool>,
    api_keys: Arc<Mutex<Vec<storage::ApiKey>>>,
    ephemeral_keys: Arc<Mutex<Vec<storage::EphemeralKey>>>,
    cards_info: Arc<Mutex<Vec<storage::CardInfo>>>,
    events: Arc<Mutex<Vec<storage::Event>>>,
    disputes: Arc<Mutex<Vec<storage::Dispute>>>,
    lockers: Arc<Mutex<Vec<storage::LockerMockUp>>>,
    mandates: Arc<Mutex<Vec<storage::Mandate>>>,
    merchant_key_store: Arc<Mutex<Vec<storage::MerchantKeyStore>>>,
}

impl MockDb {
    pub async fn new(redis: &crate::configs::settings::Settings) -> Self {
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
            redis: Arc::new(crate::connection::redis_connection(redis).await),
            api_keys: Default::default(),
            ephemeral_keys: Default::default(),
            cards_info: Default::default(),
            events: Default::default(),
            disputes: Default::default(),
            lockers: Default::default(),
            mandates: Default::default(),
            merchant_key_store: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {}

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
    fn get_redis_conn(
        &self,
    ) -> Result<
        Arc<redis_interface::RedisConnectionPool>,
        error_stack::Report<redis_interface::errors::RedisError>,
    > {
        Ok(self.redis.clone())
    }
}

dyn_clone::clone_trait_object!(StorageInterface);
