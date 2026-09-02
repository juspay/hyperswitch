//! Shared application state.

use std::{collections::HashMap, sync::Arc};

use external_services::chat_service::{slack::SlackClient, xyne::XyneClient};
use hyperswitch_interfaces::{
    secrets_interface::secret_state::{RawSecret, SecuredSecret},
    types::Proxy,
};

use crate::{
    domain::notifier::{
        chat::{ChatClientNotifier, ChatNotifier, LogChatNotifier},
        email::{EmailNotifier, LogEmailNotifier},
        Registry,
    },
    errors::ConfigurationError,
    logger, secrets_transformers,
    settings::{ChatDestination, ChatSettings, EmailSettings, Settings},
};

/// Everything a request handler needs, cloned per worker.
///
/// The registries are built once here rather than per request. A chat destination holds a
/// validated endpoint and a connection-pool-backed client, so building it per request would move a
/// configuration failure out of boot and into the delivery path — which is the one place a
/// service like this must not be discovering problems.
#[derive(Clone)]
pub struct AppState {
    /// The resolved configuration.
    pub conf: Arc<Settings<RawSecret>>,
    /// Chat destinations, by the id a request names.
    pub chat: Arc<Registry<dyn ChatNotifier>>,
    /// Email destinations, by the id a request names.
    pub email: Arc<Registry<dyn EmailNotifier>>,
}

impl AppState {
    /// Build the application state, resolving secrets and destinations on the way.
    ///
    /// # Panics
    ///
    /// Panics if the secrets management client cannot be created, if a secret fails to resolve, or
    /// if a configured destination cannot be built. All three mean the service cannot serve a
    /// request correctly, so failing here is preferable to failing later under load.
    ///
    /// Having *no* destinations is not one of those cases. It is warned about and started, because
    /// a first deployment has none until credentials exist and refusing to boot would make the
    /// service undeployable before them.
    pub async fn new(conf: Settings<SecuredSecret>) -> Self {
        #[allow(clippy::expect_used)]
        let secret_management_client = conf
            .secrets_management
            .get_secret_management_client()
            .await
            .expect("Failed to create secret management client");

        let raw_conf =
            secrets_transformers::fetch_raw_secrets(conf, &*secret_management_client).await;

        #[allow(clippy::expect_used)]
        let chat = build_chat_registry(raw_conf.chat.get_inner(), &raw_conf.proxy)
            .expect("Failed to build the chat destinations");
        let email = build_email_registry(&raw_conf.email);

        if chat.is_empty() && email.is_empty() {
            logger::warn!(
                "No chat or email destinations are configured; every notify request will be \
                 rejected as an unknown destination"
            );
        } else {
            logger::info!(
                chat_destinations = chat.len(),
                email_destinations = email.len(),
                "Notifier destinations resolved"
            );
        }

        Self {
            conf: Arc::new(raw_conf),
            chat: Arc::new(chat),
            email: Arc::new(email),
        }
    }
}

/// Turn configured chat destinations into the notifiers that serve them.
///
/// A destination that cannot be built is an error rather than a skipped entry. Dropping it would
/// leave the service running and answering "unknown destination" to a destination that is very
/// much configured, which is a far worse thing to debug than a failure to start.
fn build_chat_registry(
    settings: &ChatSettings,
    proxy: &Proxy,
) -> Result<Registry<dyn ChatNotifier>, ConfigurationError> {
    let mut destinations: HashMap<String, Arc<dyn ChatNotifier>> =
        HashMap::with_capacity(settings.destinations.len());

    for (id, destination) in &settings.destinations {
        let notifier: Arc<dyn ChatNotifier> = match destination {
            ChatDestination::Xyne(config) => Arc::new(ChatClientNotifier::new(
                id.clone(),
                Arc::new(
                    XyneClient::new(config.clone(), proxy.clone()).map_err(|error| {
                        ConfigurationError::ConfigParsingError(format!(
                            "chat destination `{id}` is not usable: {error}"
                        ))
                    })?,
                ),
            )),
            ChatDestination::Slack(config) => Arc::new(ChatClientNotifier::new(
                id.clone(),
                Arc::new(
                    SlackClient::new(config.clone(), proxy.clone()).map_err(|error| {
                        ConfigurationError::ConfigParsingError(format!(
                            "chat destination `{id}` is not usable: {error}"
                        ))
                    })?,
                ),
            )),
            ChatDestination::Log => Arc::new(LogChatNotifier::new(id.clone())),
        };

        destinations.insert(id.clone(), notifier);
    }

    Ok(Registry::new(destinations))
}

/// Turn configured email destinations into the notifiers that serve them.
///
/// Every one is a log destination for now. How `alerts` reaches `external_services::email` is
/// the email transport ticket's decision, and this function is where its answer lands.
fn build_email_registry(settings: &EmailSettings) -> Registry<dyn EmailNotifier> {
    Registry::new(
        settings
            .destinations
            .keys()
            .map(|id| {
                let notifier: Arc<dyn EmailNotifier> = Arc::new(LogEmailNotifier::new(id.clone()));
                (id.clone(), notifier)
            })
            .collect(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::settings::EmailDestination;

    #[test]
    fn a_log_destination_needs_no_credential_to_build() {
        let settings = ChatSettings {
            destinations: HashMap::from([("smoke".to_owned(), ChatDestination::Log)]),
        };

        let registry = build_chat_registry(&settings, &Proxy::default()).unwrap();
        assert!(registry.get("smoke").is_some());
        assert!(registry.get("missing").is_none());
    }

    /// A destination the endpoint rejects must stop the boot, not quietly vanish from the
    /// registry and resurface as "unknown destination" on the first alert.
    #[test]
    fn a_chat_destination_with_no_channel_fails_the_boot() {
        let config: external_services::chat_service::xyne::XyneConfig =
            serde_json::from_value(serde_json::json!({ "app_jwt": "jwt", "channel": "  " }))
                .unwrap();

        let settings = ChatSettings {
            destinations: HashMap::from([("sr_alerts".to_owned(), ChatDestination::Xyne(config))]),
        };

        let error = build_chat_registry(&settings, &Proxy::default()).unwrap_err();
        assert!(error.to_string().contains("sr_alerts"));
    }

    #[test]
    fn email_destinations_become_notifiers() {
        let settings = EmailSettings {
            destinations: HashMap::from([(
                "oncall".to_owned(),
                EmailDestination {
                    to: serde_json::from_value(serde_json::json!("oncall@example.com")).unwrap(),
                },
            )]),
        };

        assert_eq!(build_email_registry(&settings).len(), 1);
    }

    #[test]
    fn an_empty_registry_reports_itself_as_empty() {
        assert!(build_email_registry(&EmailSettings::default()).is_empty());
        assert!(
            build_chat_registry(&ChatSettings::default(), &Proxy::default())
                .unwrap()
                .is_empty()
        );
    }
}
