#[cfg(feature = "v1")]
use common_utils::errors::CustomResult;
use common_utils::types::keymanager;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::merchant_account;
use hyperswitch_domain_models::{
    cards_info, callback_mapper,connector_endpoints, customer, locker_mock_up, merchant_connector_account,
    merchant_key_store, payment_methods as pm_domain,
};
use hyperswitch_interfaces::{
    configs,
    secrets_interface::secret_state::{RawSecret, SecretState, SecretStateContainer},
};
use router_env::request_id::RequestId;
use scheduler::db::process_tracker;
use storage_impl::{errors, kv_router_store::KVRouterStore, DatabaseStore, MockDb, RouterStore};

#[async_trait::async_trait]
pub trait PaymentMethodsStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + pm_domain::PaymentMethodInterface<Error = errors::StorageError>
    + callback_mapper::CallbackMapperInterface<Error = errors::StorageError>
    + cards_info::CardsInfoInterface<Error = errors::StorageError>
    + customer::CustomerInterface<Error = errors::StorageError>
    + merchant_key_store::MerchantKeyStoreInterface<Error = errors::StorageError>
    + merchant_connector_account::MerchantConnectorAccountInterface<Error = errors::StorageError>
    + locker_mock_up::LockerMockUpInterface<Error = errors::StorageError>
    + process_tracker::ProcessTrackerInterface
    + 'static
{
}
dyn_clone::clone_trait_object!(PaymentMethodsStorageInterface);

#[async_trait::async_trait]
impl PaymentMethodsStorageInterface for MockDb {}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> PaymentMethodsStorageInterface for RouterStore<T> where
    RouterStore<T>: scheduler::ProcessTrackerInterface
{
}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> PaymentMethodsStorageInterface for KVRouterStore<T> where
    KVRouterStore<T>: scheduler::ProcessTrackerInterface
{
}

#[derive(Clone)]
pub struct PaymentMethodsConfig<S: SecretState> {
    pub locker: configs::Locker,
    pub jwekey: SecretStateContainer<configs::Jwekey, S>,
    pub proxy: hyperswitch_interfaces::types::Proxy,
    pub connectors: connector_endpoints::Connectors,
    pub network_tokenization_service:
        Option<SecretStateContainer<configs::NetworkTokenizationService, S>>,
}

pub struct PaymentMethodsState {
    pub store: Box<dyn PaymentMethodsStorageInterface>,
    pub conf: PaymentMethodsConfig<RawSecret>,
    pub tenant: configs::Tenant,
    pub api_client: Box<dyn hyperswitch_interfaces::api_client::ApiClient>,
    pub request_id: Option<RequestId>,
    pub key_store: Option<merchant_key_store::MerchantKeyStore>,
    pub event_handler: Box<dyn hyperswitch_interfaces::events::EventHandlerInterface>,
    pub key_manager_state: keymanager::KeyManagerState,
}
impl From<&PaymentMethodsState> for keymanager::KeyManagerState {
    fn from(state: &PaymentMethodsState) -> Self {
        state.key_manager_state.clone()
    }
}
#[cfg(feature = "v1")]
impl PaymentMethodsState {
    pub async fn find_payment_method(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_account: &merchant_account::MerchantAccount,
        payment_method_id: String,
    ) -> CustomResult<pm_domain::PaymentMethod, errors::StorageError> {
        let db = &*self.store;

        match db
            .find_payment_method(
                key_store,
                &payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
        {
            Err(err) if err.current_context().is_db_not_found() => {
                db.find_payment_method_by_locker_id(
                    key_store,
                    &payment_method_id,
                    merchant_account.storage_scheme,
                )
                .await
            }
            Ok(pm) => Ok(pm),
            Err(err) => Err(err),
        }
    }
}

impl hyperswitch_interfaces::api_client::ApiClientWrapper for PaymentMethodsState {
    fn get_proxy(&self) -> hyperswitch_interfaces::types::Proxy {
        self.conf.proxy.clone()
    }

    fn get_api_client(&self) -> &dyn hyperswitch_interfaces::api_client::ApiClient {
        self.api_client.as_ref()
    }

    fn get_request_id(&self) -> Option<RequestId> {
        self.api_client.get_request_id()
    }
    fn get_request_id_str(&self) -> Option<String> {
        self.api_client
            .get_request_id()
            .map(|req_id| req_id.to_string())
    }

    fn get_tenant(&self) -> configs::Tenant {
        self.tenant.clone()
    }

    fn get_connectors(&self) -> configs::Connectors {
        self.conf.connectors.clone()
    }

    fn event_handler(&self) -> &dyn hyperswitch_interfaces::events::EventHandlerInterface {
        self.event_handler.as_ref()
    }
}
