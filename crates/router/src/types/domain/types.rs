use ::payment_methods::state as pm_state;
use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};
use hyperswitch_interfaces::configs;

use crate::{
    routes::app,
    types::{api as api_types, ForeignFrom},
};

impl ForeignFrom<(&app::AppState, configs::Tenant)> for KeyManagerState {
    fn foreign_from((app_state, tenant): (&app::AppState, configs::Tenant)) -> Self {
        let conf = app_state.conf.key_manager.get_inner();
        Self {
            global_tenant_id: app_state.conf.multitenancy.global_tenant.tenant_id.clone(),
            tenant_id: tenant.tenant_id.clone(),
            enabled: conf.enabled,
            url: conf.url.clone(),
            client_idle_timeout: app_state.conf.proxy.idle_pool_connection_timeout,
            #[cfg(feature = "km_forward_x_request_id")]
            request_id: app_state.request_id.clone(),
            #[cfg(feature = "keymanager_mtls")]
            cert: conf.cert.clone(),
            #[cfg(feature = "keymanager_mtls")]
            ca: conf.ca.clone(),
            infra_values: app::AppState::process_env_mappings(app_state.conf.infra_values.clone()),
        }
    }
}
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
            request_id: state.request_id.clone(),
            #[cfg(feature = "keymanager_mtls")]
            cert: conf.cert.clone(),
            #[cfg(feature = "keymanager_mtls")]
            ca: conf.ca.clone(),
            infra_values: app::AppState::process_env_mappings(state.conf.infra_values.clone()),
        }
    }
}

impl From<&app::SessionState> for pm_state::PaymentMethodsState {
    fn from(state: &app::SessionState) -> Self {
        Self {
            store: state.store.get_payment_methods_store(),
            key_store: None,
            key_manager_state: state.into(),
        }
    }
}

pub struct ConnectorConversionHandler;

impl hyperswitch_interfaces::api_client::ConnectorConverter for ConnectorConversionHandler {
    fn get_connector_enum_by_name(
        &self,
        connector: &str,
    ) -> common_utils::errors::CustomResult<
        hyperswitch_interfaces::connector_integration_interface::ConnectorEnum,
        hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse,
    > {
        api_types::ConnectorData::convert_connector(connector)
    }
}

impl From<app::SessionState> for subscriptions::state::SubscriptionState {
    fn from(state: app::SessionState) -> Self {
        Self {
            store: state.store.get_subscription_store(),
            key_store: None,
            key_manager_state: (&state).into(),
            api_client: state.api_client.clone(),
            conf: subscriptions::state::SubscriptionConfig {
                proxy: state.conf.proxy.clone(),
                internal_merchant_id_profile_id_auth: state
                    .conf
                    .internal_merchant_id_profile_id_auth
                    .clone(),
                internal_services: state.conf.internal_services.clone(),
                connectors: state.conf.connectors.clone(),
            },
            tenant: state.tenant.clone(),
            event_handler: Box::new(state.event_handler.clone()),
            connector_converter: Box::new(ConnectorConversionHandler),
        }
    }
}
