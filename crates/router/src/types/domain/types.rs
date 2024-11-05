use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    crypto_operation, AsyncLift, CryptoOperation, Lift, OptionalEncryptableJsonType,
};

impl From<&crate::SessionState> for KeyManagerState {
    fn from(state: &crate::SessionState) -> Self {
        let conf = state.conf.key_manager.get_inner();
        Self {
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
