pub mod address;
pub mod analytics_report;
pub mod api_keys;
pub mod authentication;
pub mod authorization;
pub mod batch_blocklist_job;
pub mod blocklist;
pub mod blocklist_fingerprint;
pub mod blocklist_lookup;
pub mod business_profile;
pub mod callback_mapper;
pub mod capture;
pub mod card_issuer;
pub mod client_secret;
pub mod configs;
pub mod customers;
pub mod dashboard_metadata;
pub mod dispute;
pub mod dynamic_routing_stats;
pub mod events;
pub mod file;
pub mod fraud_check;
pub mod generic_link;
pub mod gsm;
pub mod health_check;
pub mod hierarchical_resource;
pub mod kafka_store;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_link;
pub mod payment_method_session;
pub mod refund;
pub mod relay;
pub mod reverse_lookup;
pub mod role;
pub mod routing_algorithm;
pub mod unified_translations;
pub mod user;
pub mod user_authentication_method;
pub mod user_key_store;
pub mod user_role;

use ::payment_methods::state::PaymentMethodsStorageInterface;
use common_utils::{id_type, types::keymanager::KeyManagerState};
use diesel_models::{
    fraud_check::{FraudCheck, FraudCheckUpdate},
    organization::{Organization, OrganizationNew, OrganizationUpdate},
};
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::payouts::{
    payout_attempt::TrackedPayoutAttemptInterface, payouts::TrackedPayoutsInterface,
};
use hyperswitch_domain_models::{
    authentication::TrackedAuthenticationInterface,
    card_issuer::TrackedCardIssuersInterface,
    cards_info::TrackedCardsInfoInterface,
    master_key::MasterKeyInterface,
    payment_methods::TrackedPaymentMethodInterface,
    payments::{
        payment_attempt::TrackedPaymentAttemptInterface,
        payment_intent::TrackedPaymentIntentInterface,
    },
};
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};
use redis_interface::errors::RedisError;
use router_env::logger;
#[cfg(feature = "v2")]
use storage_impl::revenue_recovery_retry_stats;
use storage_impl::{
    errors::StorageError,
    redis::{cache::CacheInterface, kv_store::RedisConnInterface},
    tokenization, MockDb,
};

pub use self::kafka_store::KafkaStore;
use self::{
    fraud_check::FraudCheckInterface,
    organization::{OrganizationInterface, TrackedOrganizationInterface},
};
pub use crate::{
    core::errors::{self, ProcessTrackerError},
    errors::CustomResult,
    services::{
        kafka::{KafkaError, KafkaProducer, MQResult},
        Store,
    },
    types::{
        domain,
        storage::{self},
        AccessToken,
    },
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
    + address::TrackedAddressInterface
    + api_keys::TrackedApiKeyInterface
    + blocklist_lookup::TrackedBlocklistLookupInterface
    + hyperswitch_domain_models::configs::TrackedConfigInterface<Error = StorageError>
    + capture::TrackedCaptureInterface
    + hyperswitch_domain_models::customer::TrackedCustomerInterface<Error = StorageError>
    + dispute::TrackedDisputeInterface
    + client_secret::TrackedClientSecretInterface
    + events::TrackedEventInterface
    + file::TrackedFileMetadataInterface
    + fraud_check::TrackedFraudCheckInterface
    + locker_mock_up::TrackedLockerMockUpInterface
    + mandate::TrackedMandateInterface
    + merchant_connector_account::ConnectorAccessToken
    + TrackedPaymentAttemptInterface<Error = StorageError>
    + TrackedPaymentIntentInterface<Error = StorageError>
    + TrackedPaymentMethodInterface<Error = StorageError>
    + analytics_report::TrackedAnalyticsReportInterface
    + blocklist::TrackedBlocklistInterface
    + blocklist_fingerprint::TrackedBlocklistFingerprintInterface
    + batch_blocklist_job::TrackedBatchBlocklistJobInterface
    + TrackedCardIssuersInterface<Error = StorageError>
    + dynamic_routing_stats::TrackedDynamicRoutingStatsInterface
    + scheduler::SchedulerInterface
    + TrackedPayoutAttemptInterface<Error = StorageError>
    + TrackedPayoutsInterface<Error = StorageError>
    + refund::TrackedRefundInterface
    + reverse_lookup::TrackedReverseLookupInterface
    + TrackedCardsInfoInterface<Error = StorageError>
    + MasterKeyInterface
    + payment_link::TrackedPaymentLinkInterface
    + RedisConnInterface
    + CacheInterface
    + RequestIdStore
    + routing_algorithm::TrackedRoutingAlgorithmInterface
    + gsm::TrackedGsmInterface
    + unified_translations::TrackedUnifiedTranslationsInterface
    + authorization::TrackedAuthorizationInterface
    + user::sample_data::TrackedBatchSampleDataInterface
    + health_check::TrackedHealthCheckDbInterface
    + user_authentication_method::TrackedUserAuthenticationMethodInterface
    + TrackedAuthenticationInterface<Error = StorageError>
    + generic_link::TrackedGenericLinkInterface
    + relay::TrackedRelayInterface
    + user::theme::TrackedThemeInterface
    + payment_method_session::TrackedPaymentMethodsSessionInterface
    + tokenization::TrackedTokenizationInterface
    + callback_mapper::TrackedCallbackMapperInterface
    + hyperswitch_domain_models::subscription::TrackedSubscriptionInterface<Error = StorageError>
    + hyperswitch_domain_models::invoice::TrackedInvoiceInterface<Error = StorageError>
    + 'static
{
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface>;
    #[cfg(feature = "v2")]
    fn get_revenue_recovery_retry_stats_store(
        &self,
    ) -> Box<
        dyn revenue_recovery_retry_stats::RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    >;
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface>;
    fn get_subscription_store(&self)
        -> Box<dyn subscriptions::state::SubscriptionStorageInterface>;
    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static>;
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState);
}

#[cfg(feature = "deja")]
#[async_trait::async_trait]
pub trait GlobalStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + user::UserInterface
    + user_role::UserRoleInterface
    + user_key_store::UserKeyStoreInterface
    + role::RoleInterface
    + RedisConnInterface
    + CacheInterface
    + RequestIdStore
    + 'static
{
    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static>;
}

#[cfg(not(feature = "deja"))]
#[async_trait::async_trait]
pub trait GlobalStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + user::UserInterface
    + user_role::UserRoleInterface
    + user_key_store::UserKeyStoreInterface
    + role::RoleInterface
    + RedisConnInterface
    + CacheInterface
    + RequestIdStore
    + 'static
{
    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static>;
}

#[cfg(feature = "deja")]
#[async_trait::async_trait]
pub trait AccountsStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + TrackedOrganizationInterface
    + merchant_account::TrackedMerchantAccountInterface<Error = StorageError>
    + business_profile::TrackedProfileInterface<Error = StorageError>
    + merchant_connector_account::TrackedMerchantConnectorAccountInterface<Error = StorageError>
    + merchant_key_store::TrackedMerchantKeyStoreInterface<Error = StorageError>
    + hierarchical_resource::TrackedHierarchicalResourceInterface<Error = StorageError>
    + dashboard_metadata::TrackedDashboardMetadataInterface
    + RequestIdStore
    + 'static
{
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState);
}

#[cfg(not(feature = "deja"))]
#[async_trait::async_trait]
pub trait AccountsStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + TrackedOrganizationInterface
    + merchant_account::TrackedMerchantAccountInterface<Error = StorageError>
    + business_profile::TrackedProfileInterface<Error = StorageError>
    + merchant_connector_account::TrackedMerchantConnectorAccountInterface<Error = StorageError>
    + merchant_key_store::TrackedMerchantKeyStoreInterface<Error = StorageError>
    + hierarchical_resource::TrackedHierarchicalResourceInterface<Error = StorageError>
    + dashboard_metadata::TrackedDashboardMetadataInterface
    + RequestIdStore
    + 'static
{
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState);
}

pub trait CommonStorageInterface:
    StorageInterface
    + GlobalStorageInterface
    + AccountsStorageInterface
    + PaymentMethodsStorageInterface
{
    fn get_storage_interface(&self) -> Box<dyn StorageInterface>;
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface>;
    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface>;
}

#[async_trait::async_trait]
impl StorageInterface for Store {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
    #[cfg(feature = "v2")]
    fn get_revenue_recovery_retry_stats_store(
        &self,
    ) -> Box<
        dyn revenue_recovery_retry_stats::RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    > {
        Box::new(self.clone())
    }
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface> {
        Box::new(self.clone())
    }

    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static> {
        Box::new(self.clone())
    }

    fn get_subscription_store(
        &self,
    ) -> Box<dyn subscriptions::state::SubscriptionStorageInterface> {
        Box::new(self.clone())
    }

    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState) {
        self.key_manager_state = Some(key_manager_state.clone());
        #[cfg(feature = "kv_store")]
        self.router_store.set_key_manager_state(key_manager_state);
    }
}

#[async_trait::async_trait]
impl GlobalStorageInterface for Store {
    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static> {
        Box::new(self.clone())
    }
}

impl AccountsStorageInterface for Store {
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState) {
        StorageInterface::set_key_manager_state(self, key_manager_state);
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
    #[cfg(feature = "v2")]
    fn get_revenue_recovery_retry_stats_store(
        &self,
    ) -> Box<
        dyn revenue_recovery_retry_stats::RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    > {
        Box::new(self.clone())
    }
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface> {
        Box::new(self.clone())
    }

    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static> {
        Box::new(self.clone())
    }
    fn get_subscription_store(
        &self,
    ) -> Box<dyn subscriptions::state::SubscriptionStorageInterface> {
        Box::new(self.clone())
    }
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState) {
        self.key_manager_state = Some(key_manager_state);
    }
}

#[async_trait::async_trait]
impl GlobalStorageInterface for MockDb {
    fn get_cache_store(&self) -> Box<dyn RedisConnInterface + Send + Sync + 'static> {
        Box::new(self.clone())
    }
}

impl AccountsStorageInterface for MockDb {
    fn set_key_manager_state(&mut self, key_manager_state: KeyManagerState) {
        StorageInterface::set_key_manager_state(self, key_manager_state);
    }
}

impl CommonStorageInterface for MockDb {
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface> {
        Box::new(self.clone())
    }
    fn get_storage_interface(&self) -> Box<dyn StorageInterface> {
        Box::new(self.clone())
    }

    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface> {
        Box::new(self.clone())
    }
}

impl CommonStorageInterface for Store {
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface> {
        Box::new(self.clone())
    }
    fn get_storage_interface(&self) -> Box<dyn StorageInterface> {
        Box::new(self.clone())
    }
    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface> {
        Box::new(self.clone())
    }
}

pub trait RequestIdStore {
    fn add_request_id(&mut self, _request_id: String) {}
    fn get_request_id(&self) -> Option<String> {
        None
    }

    /// Hand this store the session's read ledger, so what it reads is weighed
    /// against what the session already read. One ledger per session, attached
    /// to each of its stores.
    fn attach_read_ledger(
        &mut self,
        _read_ledger: std::sync::Arc<common_utils::read_ledger::ReadLedger>,
    ) {
    }

    fn get_read_ledger(&self) -> Option<std::sync::Arc<common_utils::read_ledger::ReadLedger>> {
        None
    }
}

impl RequestIdStore for MockDb {}

impl RequestIdStore for Store {
    fn add_request_id(&mut self, request_id: String) {
        // During deja replay, also stamp the inner RouterStore in KV builds because
        // PostgresOnly-delegated operations route through it.
        #[cfg(all(feature = "kv_store", feature = "deja"))]
        {
            self.router_store.request_id = Some(request_id.clone());
        }
        self.request_id = Some(request_id.clone());
        #[cfg(feature = "kv_store")]
        {
            self.router_store.request_id = Some(request_id.clone());
        }
        self.update_key_manager_request_id(request_id);
    }

    fn get_request_id(&self) -> Option<String> {
        self.request_id.clone()
    }

    fn attach_read_ledger(
        &mut self,
        read_ledger: std::sync::Arc<common_utils::read_ledger::ReadLedger>,
    ) {
        // Both direct and KV-delegated reads use the ledger in `RouterStore`.
        #[cfg(feature = "kv_store")]
        {
            self.router_store.read_ledger = Some(read_ledger);
        }
        #[cfg(not(feature = "kv_store"))]
        {
            self.read_ledger = Some(read_ledger);
        }
    }

    fn get_read_ledger(&self) -> Option<std::sync::Arc<common_utils::read_ledger::ReadLedger>> {
        #[cfg(feature = "kv_store")]
        {
            self.router_store.read_ledger.clone()
        }
        #[cfg(not(feature = "kv_store"))]
        {
            self.read_ledger.clone()
        }
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &'static str,
) -> CustomResult<T, RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(RedisError::JsonDeserializationFailed)
}

dyn_clone::clone_trait_object!(StorageInterface);
dyn_clone::clone_trait_object!(GlobalStorageInterface);
dyn_clone::clone_trait_object!(AccountsStorageInterface);

impl RequestIdStore for KafkaStore {
    fn add_request_id(&mut self, request_id: String) {
        self.diesel_store.add_request_id(request_id)
    }

    fn attach_read_ledger(
        &mut self,
        read_ledger: std::sync::Arc<common_utils::read_ledger::ReadLedger>,
    ) {
        self.diesel_store.attach_read_ledger(read_ledger)
    }

    fn get_read_ledger(&self) -> Option<std::sync::Arc<common_utils::read_ledger::ReadLedger>> {
        self.diesel_store.get_read_ledger()
    }
}

#[async_trait::async_trait]
impl FraudCheckInterface for KafkaStore {
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self.diesel_store.insert_fraud_check_response(new).await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message = "Failed to log analytics event for fraud check", error_message = ?er);
        }
        Ok(frm)
    }
    async fn update_fraud_check_response_with_frm_id(
        &self,
        this: FraudCheck,
        fraud_check: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self
            .diesel_store
            .update_fraud_check_response_with_frm_id(this, fraud_check)
            .await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for fraud check {frm:?}", error_message=?er)
        }
        Ok(frm)
    }
    async fn find_fraud_check_by_frm_id(
        &self,
        frm_id: String,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self.diesel_store.find_fraud_check_by_frm_id(frm_id).await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for fraud check {frm:?}", error_message=?er)
        }
        Ok(frm)
    }
}

#[async_trait::async_trait]
impl OrganizationInterface for KafkaStore {
    async fn insert_organization(
        &self,
        organization: OrganizationNew,
    ) -> CustomResult<Organization, StorageError> {
        OrganizationInterface::insert_organization(&self.diesel_store, organization).await
    }
    async fn find_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Organization, StorageError> {
        OrganizationInterface::find_organization_by_org_id(&self.diesel_store, org_id).await
    }

    async fn update_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
        update: OrganizationUpdate,
    ) -> CustomResult<Organization, StorageError> {
        OrganizationInterface::update_organization_by_org_id(&self.diesel_store, org_id, update)
            .await
    }
}
