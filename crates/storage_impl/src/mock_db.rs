use std::sync::Arc;

use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use diesel_models as store;
use error_stack::ResultExt;
use futures::lock::{Mutex, MutexGuard};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    errors::StorageError,
    merchant_key_store::MerchantKeyStore,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use redis_interface::RedisSettings;

use crate::redis::RedisStore;

pub mod payment_attempt;
pub mod payment_intent;
#[cfg(feature = "payouts")]
pub mod payout_attempt;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod redis_conn;
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};

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
    pub redis: Arc<RedisStore>,
    pub api_keys: Arc<Mutex<Vec<store::ApiKey>>>,
    pub ephemeral_keys: Arc<Mutex<Vec<store::EphemeralKey>>>,
    pub cards_info: Arc<Mutex<Vec<store::CardInfo>>>,
    pub events: Arc<Mutex<Vec<store::Event>>>,
    pub disputes: Arc<Mutex<Vec<store::Dispute>>>,
    pub lockers: Arc<Mutex<Vec<store::LockerMockUp>>>,
    pub mandates: Arc<Mutex<Vec<store::Mandate>>>,
    pub captures: Arc<Mutex<Vec<store::capture::Capture>>>,
    pub merchant_key_store: Arc<Mutex<Vec<store::merchant_key_store::MerchantKeyStore>>>,
    pub business_profiles: Arc<Mutex<Vec<store::business_profile::Profile>>>,
    pub reverse_lookups: Arc<Mutex<Vec<store::ReverseLookup>>>,
    pub payment_link: Arc<Mutex<Vec<store::payment_link::PaymentLink>>>,
    pub organizations: Arc<Mutex<Vec<store::organization::Organization>>>,
    pub users: Arc<Mutex<Vec<store::user::User>>>,
    pub user_roles: Arc<Mutex<Vec<store::user_role::UserRole>>>,
    pub authorizations: Arc<Mutex<Vec<store::authorization::Authorization>>>,
    pub dashboard_metadata: Arc<Mutex<Vec<store::user::dashboard_metadata::DashboardMetadata>>>,
    #[cfg(feature = "payouts")]
    pub payout_attempt: Arc<Mutex<Vec<store::payout_attempt::PayoutAttempt>>>,
    #[cfg(feature = "payouts")]
    pub payouts: Arc<Mutex<Vec<store::payouts::Payouts>>>,
    pub authentications: Arc<Mutex<Vec<store::authentication::Authentication>>>,
    pub roles: Arc<Mutex<Vec<store::role::Role>>>,
    pub user_key_store: Arc<Mutex<Vec<store::user_key_store::UserKeyStore>>>,
    pub user_authentication_methods:
        Arc<Mutex<Vec<store::user_authentication_method::UserAuthenticationMethod>>>,
    pub themes: Arc<Mutex<Vec<store::user::theme::Theme>>>,
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
            users: Default::default(),
            user_roles: Default::default(),
            authorizations: Default::default(),
            dashboard_metadata: Default::default(),
            #[cfg(feature = "payouts")]
            payout_attempt: Default::default(),
            #[cfg(feature = "payouts")]
            payouts: Default::default(),
            authentications: Default::default(),
            roles: Default::default(),
            user_key_store: Default::default(),
            user_authentication_methods: Default::default(),
            themes: Default::default(),
        })
    }

    pub async fn find_resource<D, R>(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        resources: MutexGuard<'_, Vec<D>>,
        filter_fn: impl Fn(&&D) -> bool,
        error_message: String,
    ) -> CustomResult<R, StorageError>
    where
        D: Sync + ReverseConversion<R> + Clone,
        R: Conversion,
    {
        let resource = resources.iter().find(filter_fn).cloned();
        match resource {
            Some(res) => Ok(res
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?),
            None => Err(StorageError::ValueNotFound(error_message).into()),
        }
    }

    pub async fn find_resources<D, R>(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        resources: MutexGuard<'_, Vec<D>>,
        filter_fn: impl Fn(&&D) -> bool,
        error_message: String,
    ) -> CustomResult<Vec<R>, StorageError>
    where
        D: Sync + ReverseConversion<R> + Clone,
        R: Conversion,
    {
        let resources: Vec<_> = resources.iter().filter(filter_fn).cloned().collect();
        if resources.is_empty() {
            Err(StorageError::ValueNotFound(error_message).into())
        } else {
            let pm_futures = resources
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_resources = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_resources)
        }
    }

    pub async fn update_resource<D, R>(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        mut resources: MutexGuard<'_, Vec<D>>,
        resource_updated: D,
        filter_fn: impl Fn(&&mut D) -> bool,
        error_message: String,
    ) -> CustomResult<R, StorageError>
    where
        D: Sync + ReverseConversion<R> + Clone,
        R: Conversion,
    {
        if let Some(pm) = resources.iter_mut().find(filter_fn) {
            *pm = resource_updated.clone();
            let result = resource_updated
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?;
            Ok(result)
        } else {
            Err(StorageError::ValueNotFound(error_message).into())
        }
    }
}

#[cfg(not(feature = "payouts"))]
impl PayoutsInterface for MockDb {}

#[cfg(not(feature = "payouts"))]
impl PayoutAttemptInterface for MockDb {}
