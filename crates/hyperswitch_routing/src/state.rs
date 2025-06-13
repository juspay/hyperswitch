use crate::errors::RouterResponse;
use crate::errors::RouterResult;
use api_models::enums as api_enums;
use common_utils::{request, types::keymanager::KeyManagerState};
use external_services::grpc_client::{GrpcClients, GrpcHeaders};
use hyperswitch_domain_models::{
    configs::ConfigInterface,
    db::{
        business_profile::ProfileInterface, dynamic_routing_stats::DynamicRoutingStatsInterface,
        routing_algorithm::RoutingAlgorithmInterface,
    },
    merchant_account, merchant_connector_account,
    merchant_key_store::MerchantKeyStore,
};
use hyperswitch_interfaces::session_connector_data;
use router_env::tracing_actix_web::RequestId;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
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
    + DynamicRoutingStatsInterface<Error = errors::StorageError>
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
        profile_id: &common_utils::id_type::ProfileId,
    ) -> RouterResponse<Vec<api_models::admin::MerchantConnectorResponse>>;
    async fn get_disabled_merchant_connector_accounts(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> RouterResult<merchant_connector_account::MerchantConnectorAccounts>;
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> RouterResult<merchant_connector_account::MerchantConnectorAccount>;
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

pub trait ConnectorHandlerInterface: dyn_clone::DynClone + Send + Sync {
    fn get_connector_by_name(
        &self,
        connector_name: String,
        get_token: session_connector_data::GetToken,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> RouterResult<session_connector_data::ConnectorData>;
}

dyn_clone::clone_trait_object!(ConnectorHandlerInterface);

#[derive(Clone)]
pub struct RoutingState<'a> {
    pub store: Box<dyn RoutingStorageInterface>,
    pub conf: RoutingConfig,
    pub api_client: Box<dyn request::ApiClient>,
    pub connector_filters: HashMap<String, kgraph_utils::types::PaymentMethodFilters>,
    pub mca_handler: Box<dyn MerchantConnectorInterface + 'a>,
    pub merchant_account_handler: Box<dyn MerchantAccountInterface + 'a>,
    pub connector_handler: Box<dyn ConnectorHandlerInterface + 'a>,
    pub key_manager_state: KeyManagerState,
    pub tenant: Tenant,
    pub request_id: Option<RequestId>,
    #[cfg(feature = "dynamic_routing")]
    pub grpc_client: Arc<GrpcClients>,
}

impl RoutingState<'_> {
    pub fn get_grpc_headers(&self) -> GrpcHeaders {
        GrpcHeaders {
            tenant_id: self.tenant.tenant_id.get_string_repr().to_string(),
            request_id: self.request_id.map(|req_id| (*req_id).to_string()),
        }
    }
}

#[derive(Clone)]
pub struct RoutingConfig {
    pub proxy: request::Proxy,
    pub open_router: OpenRouter,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct OpenRouter {
    pub enabled: bool,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub redis_key_prefix: String,
    pub tenant_id: common_utils::id_type::TenantId,
}

impl<'a> From<&RoutingState<'a>> for KeyManagerState {
    fn from(state: &RoutingState<'a>) -> Self {
        state.key_manager_state.clone()
    }
}
