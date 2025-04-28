use ::payment_methods::{configs::settings as pm_settings, state as pm_state};
use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};

use crate::services::api::ApiCaller;

impl From<&crate::SessionState> for KeyManagerState {
    fn from(state: &crate::SessionState) -> Self {
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

impl From<&crate::SessionState> for pm_state::PaymentMethodsState {
    fn from(state: &crate::SessionState) -> Self {
        Self {
            store: state.store.get_payment_methods_store(),
            key_store: None,
            key_manager_state: state.into(),
            conf: pm_settings::PaymentMethodsConfig {
                network_tokenization_supported_card_networks: state
                    .conf
                    .network_tokenization_supported_card_networks
                    .clone(),
                locker: pm_settings::LockerConfig {
                    ttl_for_storage_in_secs: state.conf.locker.ttl_for_storage_in_secs,
                },
                network_tokenization_service: state.conf.network_tokenization_service.clone(),
            },
            connector_api_client: Box::new(ApiCaller::new(state.clone(), state.api_client.clone())),
        }
    }
}
