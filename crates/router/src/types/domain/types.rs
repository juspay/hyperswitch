use common_utils::types::keymanager::KeyManagerState;
pub use hyperswitch_domain_models::type_encryption::{
    batch_decrypt, batch_decrypt_optional, batch_encrypt, batch_encrypt_optional, decrypt, encrypt,
    encrypt_optional, AsyncLift, Lift, TypeEncryption,
};

impl From<&crate::SessionState> for KeyManagerState {
    fn from(state: &crate::SessionState) -> Self {
        let conf = state.conf.key_manager.get_inner();
        Self {
            url: conf.url.clone(),
            client_idle_timeout: state.conf.proxy.idle_pool_connection_timeout,
            #[cfg(feature = "keymanager_mtls")]
            cert: conf.cert.clone(),
            #[cfg(feature = "keymanager_mtls")]
            ca: conf.ca.clone(),
        }
    }
}
