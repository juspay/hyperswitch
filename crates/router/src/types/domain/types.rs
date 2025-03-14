use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};

impl From<&crate::SessionState> for KeyManagerState {
    fn from(state: &crate::SessionState) -> Self {
        let conf = state.conf.key_manager.get_inner();
        Self {
            global_tenant_id: state.conf.multitenancy.global_tenant.tenant_id.clone(),
            tenant_id: state.tenant.tenant_id.clone(),
            mtls_enabled: conf.mtls_enabled,
            enabled: conf.enabled,
            url: conf.url.clone(),
            client_idle_timeout: state.conf.proxy.idle_pool_connection_timeout,
            request_id: state.request_id,
            cert: conf.cert.clone(),
            ca: conf.ca.clone(),
        }
    }
}
