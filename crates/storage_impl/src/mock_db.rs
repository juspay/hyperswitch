use std::sync::Arc;

use data_models::{
    errors::StorageError,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use diesel_models::{self as store};
use error_stack::ResultExt;
use futures::lock::Mutex;
use redis_interface::RedisSettings;

use crate::redis::RedisStore;

pub mod payment_attempt;
pub mod payment_intent;
pub mod redis_conn;

#[derive(Clone)]
pub struct MockDb {
    pub addresses: Arc<Mutex<Vec<store::Address>>>,
    pub configs: Arc<Mutex<Vec<store::Config>>>,
    pub merchant_accounts: Arc<Mutex<Vec<store::MerchantAccount>>>,
    pub merchant_connector_accounts: Arc<Mutex<Vec<store::MerchantConnectorAccount>>>,
    pub payment_attempts: Arc<Mutex<Vec<PaymentAttempt>>>,
    pub payment_intents: Arc<Mutex<Vec<PaymentIntent>>>,
    pub payment_methods: Arc<Mutex<Vec<store::PaymentMethod>>>,
    pub customers: Arc<Mutex<Vec<store::Customer>>>,
    pub refunds: Arc<Mutex<Vec<store::Refund>>>,
    pub processes: Arc<Mutex<Vec<store::ProcessTracker>>>,
    pub connector_response: Arc<Mutex<Vec<store::ConnectorResponse>>>,
    pub redis: Arc<RedisStore>,
    pub api_keys: Arc<Mutex<Vec<store::ApiKey>>>,
    pub ephemeral_keys: Arc<Mutex<Vec<store::EphemeralKey>>>,
    pub cards_info: Arc<Mutex<Vec<store::CardInfo>>>,
    pub events: Arc<Mutex<Vec<store::Event>>>,
    pub disputes: Arc<Mutex<Vec<store::Dispute>>>,
    pub lockers: Arc<Mutex<Vec<store::LockerMockUp>>>,
    pub mandates: Arc<Mutex<Vec<store::Mandate>>>,
    pub captures: Arc<Mutex<Vec<crate::store::capture::Capture>>>,
    pub merchant_key_store: Arc<Mutex<Vec<crate::store::merchant_key_store::MerchantKeyStore>>>,
    pub business_profiles: Arc<Mutex<Vec<crate::store::business_profile::BusinessProfile>>>,
    pub reverse_lookups: Arc<Mutex<Vec<store::ReverseLookup>>>,
    pub payment_link: Arc<Mutex<Vec<store::payment_link::PaymentLink>>>,
    pub organizations: Arc<Mutex<Vec<store::organization::Organization>>>,
}

impl MockDb {
    pub async fn new(redis: &RedisSettings) -> error_stack::Result<Self, StorageError> {
        Ok(Self {
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
            redis: Arc::new(
                RedisStore::new(redis)
                    .await
                    .change_context(StorageError::InitializationError)?,
            ),
            api_keys: Default::default(),
            ephemeral_keys: Default::default(),
            cards_info: Default::default(),
            events: Default::default(),
            disputes: Default::default(),
            lockers: Default::default(),
            mandates: Default::default(),
            captures: Default::default(),
            merchant_key_store: Default::default(),
            business_profiles: Default::default(),
            reverse_lookups: Default::default(),
            payment_link: Default::default(),
            organizations: Default::default(),
        })
    }
}
