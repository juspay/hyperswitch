//! Shared application state.

use std::sync::Arc;

use hyperswitch_interfaces::secrets_interface::secret_state::{RawSecret, SecuredSecret};

use crate::{secrets_transformers, settings::Settings};

/// Everything a request handler needs, cloned per worker.
///
/// Holds only `Settings<RawSecret>` today. Channel clients (chat, email) join it as the notifier
/// gains them; they belong here rather than in each handler so that a single client — and a
/// single connection pool — is shared across the whole service.
#[derive(Clone)]
pub struct AppState {
    /// The resolved configuration.
    pub conf: Arc<Settings<RawSecret>>,
}

impl AppState {
    /// Build the application state, resolving secrets on the way.
    ///
    /// # Panics
    ///
    /// Panics if the secrets management client cannot be created, or if a secret fails to
    /// resolve. Both mean the service cannot serve a single request correctly, so failing here is
    /// preferable to failing later under load.
    pub async fn new(conf: Settings<SecuredSecret>) -> Self {
        #[allow(clippy::expect_used)]
        let secret_management_client = conf
            .secrets_management
            .get_secret_management_client()
            .await
            .expect("Failed to create secret management client");

        let raw_conf =
            secrets_transformers::fetch_raw_secrets(conf, &*secret_management_client).await;

        Self {
            conf: Arc::new(raw_conf),
        }
    }
}
