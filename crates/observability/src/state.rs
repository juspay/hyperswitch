use std::{collections::HashMap, sync::Arc};

use common_utils::external_service::NoOpEventEmitter;
use diesel_models::{DatabaseConnectionWithContext, DejaPgConnection};
use error_stack::report;
use external_services::{
    chat_service::{slack::SlackClient, xyne::XyneClient},
    email::{
        no_email::NoEmailClient, ses::AwsSes, smtp::SmtpServer, EmailClientConfigs, EmailService,
        EmailSettings as EmailClientSettings,
    },
};
use hyperswitch_interfaces::{
    secrets_interface::secret_state::{RawSecret, SecuredSecret},
    types::Proxy,
};

use crate::{
    domain::notifier::{
        chat::{ChatClientNotifier, ChatNotifier, LogChatNotifier},
        email::{EmailNotifier, EmailServiceNotifier},
        Registry,
    },
    errors::{ConfigurationError, ObservabilityError},
    logger, secrets_transformers,
    settings::{ChatDestination, ChatSettings, DatabaseSettings, EmailSettings, Settings},
};

pub type DatabasePool = bb8::Pool<async_bb8_diesel::ConnectionManager<DejaPgConnection>>;

#[derive(Clone)]
pub struct AppState {
    pub conf: Arc<Settings<RawSecret>>,
    pub chat: Arc<Registry<dyn ChatNotifier>>,
    pub email: Arc<Registry<dyn EmailNotifier>>,
    pub database: DatabasePool,
}

impl AppState {
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
        #[allow(clippy::expect_used)]
        let email = build_email_registry(&raw_conf.email, &raw_conf.proxy)
            .await
            .expect("Failed to build the email destinations");

        #[allow(clippy::expect_used)]
        let database = build_database_pool(raw_conf.database.get_inner())
            .expect("Failed to build the database connection pool");

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
            database,
        }
    }

    /// Lease a connection from the pool, ready for the query helpers in `diesel_models`.
    pub async fn database_connection(
        &self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'_>, ObservabilityError> {
        let connection = self.database.get().await.map_err(|error| {
            report!(ObservabilityError::StorageUnavailable)
                .attach_printable(format!("Failed to lease a database connection: {error}"))
        })?;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            None,
            Arc::new(NoOpEventEmitter),
        ))
    }
}

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

async fn build_email_registry(
    settings: &EmailSettings,
    proxy: &Proxy,
) -> Result<Registry<dyn EmailNotifier>, ConfigurationError> {
    if settings.destinations.is_empty() {
        return Ok(Registry::default());
    }

    let client = Arc::new(create_email_client(&settings.client, proxy).await?);

    Ok(Registry::new(
        settings
            .destinations
            .iter()
            .map(|(id, destination)| {
                let notifier: Arc<dyn EmailNotifier> = Arc::new(EmailServiceNotifier::new(
                    id.clone(),
                    Arc::clone(&client),
                    destination.to.clone(),
                    proxy.https_url.clone(),
                ));
                (id.clone(), notifier)
            })
            .collect(),
    ))
}

#[derive(Debug, Clone, Copy)]
struct LogConnectionErrors;

impl<E: std::fmt::Display> bb8::ErrorSink<E> for LogConnectionErrors {
    fn sink(&self, error: E) {
        logger::error!(%error, "Observability database connection failed");
    }

    fn boxed_clone(&self) -> Box<dyn bb8::ErrorSink<E>> {
        Box::new(*self)
    }
}

pub fn build_database_pool(
    database: &DatabaseSettings,
) -> Result<DatabasePool, ConfigurationError> {
    let manager =
        async_bb8_diesel::ConnectionManager::<DejaPgConnection>::new(database.database_url());

    Ok(bb8::Pool::builder()
        .max_size(database.pool_size)
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
        .error_sink(Box::new(LogConnectionErrors))
        .build_unchecked(manager))
}

async fn create_email_client(
    settings: &EmailClientSettings,
    proxy: &Proxy,
) -> Result<Box<dyn EmailService>, ConfigurationError> {
    Ok(match &settings.client_config {
        EmailClientConfigs::Ses { aws_ses } => {
            AwsSes::create_client(settings, aws_ses, proxy.https_url.clone())
                .await
                .map_err(|error| {
                    ConfigurationError::ConfigParsingError(format!(
                        "the SES email transport is not usable: {error}"
                    ))
                })?;

            Box::new(AwsSes::create(settings, aws_ses, proxy.https_url.clone()).await)
        }
        EmailClientConfigs::Smtp { smtp } => {
            Box::new(SmtpServer::create(settings, smtp.clone()).await)
        }
        EmailClientConfigs::NoEmailClient => Box::new(NoEmailClient::create().await),
    })
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
            ..Default::default()
        };

        let registry = build_chat_registry(&settings, &Proxy::default()).unwrap();
        assert!(registry.get("smoke").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn a_chat_destination_with_no_channel_fails_the_boot() {
        let config: external_services::chat_service::xyne::XyneConfig =
            serde_json::from_value(serde_json::json!({ "app_jwt": "jwt", "channel": "  " }))
                .unwrap();

        let settings = ChatSettings {
            destinations: HashMap::from([("sr_alerts".to_owned(), ChatDestination::Xyne(config))]),
            ..Default::default()
        };

        let error = build_chat_registry(&settings, &Proxy::default()).unwrap_err();
        assert!(error.to_string().contains("sr_alerts"));
    }

    fn email_settings_with(ids: &[&str]) -> EmailSettings {
        EmailSettings {
            client: EmailClientSettings::default(),
            destinations: ids
                .iter()
                .map(|id| {
                    (
                        (*id).to_owned(),
                        EmailDestination {
                            to: serde_json::from_value(serde_json::json!("oncall@example.com"))
                                .unwrap(),
                        },
                    )
                })
                .collect(),
        }
    }

    #[tokio::test]
    async fn email_destinations_share_one_client() {
        let registry = build_email_registry(
            &email_settings_with(&["oncall", "escalation"]),
            &Proxy::default(),
        )
        .await
        .unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.get("oncall").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[tokio::test]
    async fn an_empty_registry_reports_itself_as_empty() {
        assert!(
            build_email_registry(&EmailSettings::default(), &Proxy::default())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            build_chat_registry(&ChatSettings::default(), &Proxy::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_destination_without_an_address_fails_validation() {
        let mut settings = email_settings_with(&["oncall"]);
        settings.destinations.insert(
            "broken".to_owned(),
            EmailDestination {
                to: Default::default(),
            },
        );

        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("broken"));
    }

    #[test]
    fn an_unused_transport_is_not_validated() {
        EmailSettings::default().validate().unwrap();
    }
}
