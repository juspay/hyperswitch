use crate::core::routing;
use std::collections::HashMap;
use crate::routes::app;
use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};
use hyperswitch_routing::state as routing_state;
use crate::types::transformers::ForeignInto;

impl From<&app::SessionState> for KeyManagerState {
    fn from(state: &app::SessionState) -> Self {
        let conf = state.conf.key_manager.get_inner();
        Self {
            global_tenant_id: state.conf.multitenancy.global_tenant.tenant_id.clone(),
            tenant_id: state.tenant.tenant_id.clone(),
            enabled: conf.enabled,
            url: conf.url.clone(),
            client_idle_timeout: state.conf.proxy.idle_pool_connection_timeout,
            #[cfg(feature = "km_forward_x_request_id")]
            request_id: state.request_id,
            #[cfg(feature = "keymanager_mtls")]
            cert: conf.cert.clone(),
            #[cfg(feature = "keymanager_mtls")]
            ca: conf.ca.clone(),
        }
    }
}

impl<'a> From<&'a app::SessionState> for routing_state::RoutingState<'a> {
    fn from(state: &'a app::SessionState) -> Self {
        Self {
            store: state.store.get_routing_store(),
            conf: routing_state::RoutingConfig {
                proxy: state.conf.proxy.clone(),
                open_router: routing_state::OpenRouter {
                    enabled: state.conf.open_router.enabled,
                    url: state.conf.open_router.url.clone(),
                },
            },
            api_client: state.api_client.clone(),
            connector_filters: state
                .conf
                .pm_filters
                .0
                .clone()
                .into_iter()
                .map(|(key, value)| (key, value.foreign_into()))
                .collect::<HashMap<_, _>>(),
            mca_handler: Box::new(routing::MerchantConnectorHandler { state }),
            merchant_account_handler: Box::new(routing::MerchantAccountHandler { state }),
            connector_handler: Box::new(routing::ConnectorHandler { state }),
            key_manager_state: state.into(),
            tenant: routing_state::Tenant {
                redis_key_prefix: state.tenant.redis_key_prefix.clone(),
                tenant_id: state.tenant.tenant_id.clone(),
            },
            request_id: state.request_id,
            #[cfg(feature = "dynamic_routing")]
            grpc_client: state.grpc_client.clone(),
        }
    }
}
