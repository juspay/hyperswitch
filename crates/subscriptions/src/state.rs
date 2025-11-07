use common_utils::types::keymanager;
use hyperswitch_domain_models::{
    business_profile, configs as domain_configs, customer, invoice as invoice_domain, master_key,
    merchant_account, merchant_connector_account, merchant_key_store,
    subscription as subscription_domain,
};
use hyperswitch_interfaces::configs;
use router_env::RequestId;
use storage_impl::{errors, kv_router_store::KVRouterStore, DatabaseStore, MockDb, RouterStore};

#[async_trait::async_trait]
pub trait SubscriptionStorageInterface:
    Send
    + Sync
    + std::any::Any
    + dyn_clone::DynClone
    + master_key::MasterKeyInterface
    + scheduler::SchedulerInterface
    + subscription_domain::SubscriptionInterface<Error = errors::StorageError>
    + invoice_domain::InvoiceInterface<Error = errors::StorageError>
    + business_profile::ProfileInterface<Error = errors::StorageError>
    + domain_configs::ConfigInterface<Error = errors::StorageError>
    + customer::CustomerInterface<Error = errors::StorageError>
    + merchant_account::MerchantAccountInterface<Error = errors::StorageError>
    + merchant_key_store::MerchantKeyStoreInterface<Error = errors::StorageError>
    + merchant_connector_account::MerchantConnectorAccountInterface<Error = errors::StorageError>
    + 'static
{
}
dyn_clone::clone_trait_object!(SubscriptionStorageInterface);

#[async_trait::async_trait]
impl SubscriptionStorageInterface for MockDb {}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> SubscriptionStorageInterface for RouterStore<T> where
    Self: scheduler::SchedulerInterface + master_key::MasterKeyInterface
{
}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> SubscriptionStorageInterface for KVRouterStore<T> where
    Self: scheduler::SchedulerInterface + master_key::MasterKeyInterface
{
}

pub struct SubscriptionState {
    pub store: Box<dyn SubscriptionStorageInterface>,
    pub key_store: Option<merchant_key_store::MerchantKeyStore>,
    pub key_manager_state: keymanager::KeyManagerState,
    pub api_client: Box<dyn hyperswitch_interfaces::api_client::ApiClient>,
    pub conf: SubscriptionConfig,
    pub tenant: configs::Tenant,
    pub event_handler: Box<dyn hyperswitch_interfaces::events::EventHandlerInterface>,
    pub connector_converter: Box<dyn hyperswitch_interfaces::api_client::ConnectorConverter>,
}

#[derive(Clone)]
pub struct SubscriptionConfig {
    pub proxy: hyperswitch_interfaces::types::Proxy,
    pub internal_merchant_id_profile_id_auth: configs::InternalMerchantIdProfileIdAuthSettings,
    pub internal_services: configs::InternalServicesConfig,
    pub connectors: configs::Connectors,
}

impl From<&SubscriptionState> for keymanager::KeyManagerState {
    fn from(state: &SubscriptionState) -> Self {
        state.key_manager_state.clone()
    }
}

impl hyperswitch_interfaces::api_client::ApiClientWrapper for SubscriptionState {
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
