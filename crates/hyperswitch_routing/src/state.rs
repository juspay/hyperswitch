use crate::errors::RouterResponse;
use api_models::enums as api_enums;
use common_utils::{request, types::keymanager::KeyManagerState};
use hyperswitch_domain_models::{
    configs::ConfigInterface,
    db::{business_profile::ProfileInterface, routing_algorithm::RoutingAlgorithmInterface},
    merchant_connector_account,
    merchant_account,
    merchant_key_store::MerchantKeyStore,
};
use std::collections::HashMap;
use crate::errors::RouterResult;
use storage_impl::redis::kv_store::RedisConnInterface;
use storage_impl::{errors, kv_router_store::KVRouterStore, DatabaseStore, MockDb, RouterStore};

#[async_trait::async_trait]
pub trait RoutingStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + ProfileInterface<Error = errors::StorageError>
    + RoutingAlgorithmInterface<Error = errors::StorageError>
    + ConfigInterface<Error = errors::StorageError>
    + 'static
{
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)>;
}
dyn_clone::clone_trait_object!(RoutingStorageInterface);

#[async_trait::async_trait]
impl RoutingStorageInterface for MockDb {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> RoutingStorageInterface for RouterStore<T> {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> RoutingStorageInterface for KVRouterStore<T> {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait MerchantConnectorInterface: dyn_clone::DynClone + Send + Sync {
    async fn filter_merchant_connectors(
        &self,
        key_store: &MerchantKeyStore,
        transaction_type: &api_enums::TransactionType,
        profile_id: &common_utils::id_type::ProfileId
    ) -> RouterResponse<Vec<api_models::admin::MerchantConnectorResponse>>;
    async fn get_disabled_merchant_connector_accounts(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> RouterResult<merchant_connector_account::MerchantConnectorAccounts>;
}

dyn_clone::clone_trait_object!(MerchantConnectorInterface);

#[async_trait::async_trait]
pub trait MerchantAccountInterface: dyn_clone::DynClone + Send + Sync {
    async fn update_specific_fields_in_merchant(
        &self,
        key_store: &MerchantKeyStore,
        merchant_account_update: merchant_account::MerchantAccountUpdate,
    ) -> RouterResult<merchant_account::MerchantAccount>;
}

dyn_clone::clone_trait_object!(MerchantAccountInterface);

#[derive(Clone)]
pub struct RoutingState<'a> {
    pub store: Box<dyn RoutingStorageInterface>,
    pub conf: RoutingConfig,
    pub api_client: Box<dyn request::ApiClient>,
    pub connector_filters: HashMap<String, kgraph_utils::types::PaymentMethodFilters>,
    pub mca_handler: Box<dyn MerchantConnectorInterface + 'a>,
    pub merchant_account_handler: Box<dyn MerchantAccountInterface + 'a>,
    pub key_manager_state: KeyManagerState,
    pub tenant: Tenant,
}

#[derive(Clone)]
pub struct RoutingConfig {
    pub proxy: request::Proxy,
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub redis_key_prefix: String,
}

impl<'a> From<&RoutingState<'a>> for KeyManagerState {
    fn from(state: &RoutingState<'a>) -> Self {
        state.key_manager_state.clone()
    }
}
