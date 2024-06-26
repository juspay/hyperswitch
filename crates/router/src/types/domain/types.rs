use common_utils::types::keymanager::KeyManagerState;

pub use hyperswitch_domain_models::type_encryption::{
    decrypt, encrypt, encrypt_optional, AsyncLift, Lift, TypeEncryption,
};

impl From<&crate::SessionState> for KeyManagerState {
    fn from(state: &crate::SessionState) -> Self {
        Self {
            url: state.conf.key_manager.url.clone(),
            client_idle_timeout: state.conf.proxy.idle_pool_connection_timeout,
            #[cfg(feature = "keymanager_mtls")]
            cert: state.conf.key_manager.cert.clone(),
            #[cfg(feature = "keymanager_mtls")]
            ca: state.conf.key_manager.ca.clone(),
        }
    }
}
