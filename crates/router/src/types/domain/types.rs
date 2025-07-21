use ::payment_methods::state as pm_state;
use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};

use crate::routes::app;

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
