use std::{collections::HashMap, path::PathBuf};

use common_utils::{ext_traits::ConfigExt, pii};
use config::{Environment, File};
use external_services::{
    chat_service::{slack::SlackConfig, xyne::XyneConfig},
    email::{EmailClientConfigs, EmailSettings as EmailClientSettings},
    managers::secrets_management::SecretsManagementConfig,
};
use hyperswitch_interfaces::{
    secrets_interface::secret_state::{SecretState, SecretStateContainer, SecuredSecret},
    types::Proxy,
};
use hyperswitch_masking::{PeekInterface, Secret};
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use router_env::{env, logger};
use serde::Deserialize;

use crate::errors;

const CONFIG_FILE_NAME: &str = "observability.toml";

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings<S: SecretState> {
    pub server: Server,
    pub log: Log,
    pub auth: SecretStateContainer<AuthSettings, S>,
    pub database: SecretStateContainer<DatabaseSettings, S>,
    pub secrets_management: SecretsManagementConfig,
    pub proxy: Proxy,
    pub chat: SecretStateContainer<ChatSettings, S>,
    pub email: EmailSettings,
    pub lifecycle: LifecycleSettings,
    pub instances: InstanceSettings,
    pub mappers: MapperSettings,
}

const DEFAULT_MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

fn default_max_upload_bytes() -> usize {
    DEFAULT_MAX_UPLOAD_BYTES
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ChatSettings {
    pub destinations: HashMap<String, ChatDestination>,

    #[serde(default = "default_max_upload_bytes")]
    pub max_upload_bytes: usize,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            destinations: HashMap::new(),
            max_upload_bytes: default_max_upload_bytes(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatDestination {
    Xyne(XyneConfig),
    Slack(SlackConfig),
    Log,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EmailSettings {
    #[serde(flatten)]
    pub client: EmailClientSettings,

    pub destinations: HashMap<String, EmailDestination>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailDestination {
    pub to: pii::Email,
}

fn validate_destination_ids<T>(
    destinations: &HashMap<String, T>,
    section: &str,
) -> Result<(), errors::ConfigurationError> {
    for id in destinations.keys() {
        if id.is_empty() || id.contains("__") || id != &id.to_lowercase() {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "{section} destination id `{id}` must be lowercase, non-empty and free of `__`, \
                 so that it can be set from the environment"
            )))?
        }
    }
    Ok(())
}

impl ChatSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_destination_ids(&self.destinations, "chat")?;
        common_utils::fp_utils::when(self.max_upload_bytes == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "chat max_upload_bytes must be greater than zero".into(),
            ))
        })
    }
}

impl EmailSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_destination_ids(&self.destinations, "email")?;

        if self.destinations.is_empty() {
            return Ok(());
        }

        self.client
            .validate()
            .map_err(|error| errors::ConfigurationError::ConfigParsingError(error.to_owned()))?;

        common_utils::fp_utils::when(
            !matches!(self.client.client_config, EmailClientConfigs::NoEmailClient)
                && !crate::domain::notifier::email::is_usable_recipient(&self.client.sender_email),
            || {
                Err(errors::ConfigurationError::ConfigParsingError(
                    "email sender_email must be set when an email transport is configured".into(),
                ))
            },
        )?;

        for (id, destination) in &self.destinations {
            common_utils::fp_utils::when(
                !crate::domain::notifier::email::is_usable_recipient(&destination.to),
                || {
                    Err(errors::ConfigurationError::ConfigParsingError(format!(
                        "email destination `{id}` has no recipient address"
                    )))
                },
            )?;
        }

        Ok(())
    }
}

const DEFAULT_MAX_ENTRY_BYTES: usize = 1024 * 1024;

fn default_max_entry_bytes() -> usize {
    DEFAULT_MAX_ENTRY_BYTES
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MapperSettings {
    #[serde(default = "default_max_entry_bytes")]
    pub max_entry_bytes: usize,
}

impl Default for MapperSettings {
    fn default() -> Self {
        Self {
            max_entry_bytes: default_max_entry_bytes(),
        }
    }
}

impl MapperSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.max_entry_bytes == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "mappers max_entry_bytes must be greater than zero".into(),
            ))
        })
    }
}

const DEFAULT_MAX_ALERTS: usize = 5_000;

fn default_max_alerts() -> usize {
    DEFAULT_MAX_ALERTS
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LifecycleSettings {
    #[serde(default = "default_max_alerts")]
    pub max_alerts: usize,
}

impl Default for LifecycleSettings {
    fn default() -> Self {
        Self {
            max_alerts: default_max_alerts(),
        }
    }
}

impl LifecycleSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.max_alerts == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "lifecycle max_alerts must be greater than zero".into(),
            ))
        })
    }
}

const DEFAULT_MAX_INSTANCE_ROWS: usize = 500;

const MAX_INSTANCE_ROWS: usize = 2_000;

fn default_max_instance_rows() -> usize {
    DEFAULT_MAX_INSTANCE_ROWS
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct InstanceSettings {
    #[serde(default = "default_max_instance_rows")]
    pub max_merchants: usize,

    #[serde(default = "default_max_instance_rows")]
    pub max_dimensions: usize,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        Self {
            max_merchants: default_max_instance_rows(),
            max_dimensions: default_max_instance_rows(),
        }
    }
}

impl InstanceSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        for (name, limit) in [
            ("max_merchants", self.max_merchants),
            ("max_dimensions", self.max_dimensions),
        ] {
            common_utils::fp_utils::when(limit == 0 || limit > MAX_INSTANCE_ROWS, || {
                Err(errors::ConfigurationError::ConfigParsingError(format!(
                    "instances {name} must be between 1 and {MAX_INSTANCE_ROWS}"
                )))
            })?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AuthSettings {
    pub internal_api_key: Secret<String>,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            internal_api_key: String::new().into(),
        }
    }
}

impl AuthSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.internal_api_key.peek().is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "auth internal_api_key must not be empty".into(),
            ))
        })
    }
}

const DEFAULT_POOL_SIZE: u32 = 5;

const DEFAULT_CONNECTION_TIMEOUT: u64 = 10;

fn default_pool_size() -> u32 {
    DEFAULT_POOL_SIZE
}

fn default_connection_timeout() -> u64 {
    DEFAULT_CONNECTION_TIMEOUT
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub username: String,
    pub password: Secret<String>,
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            host: String::default(),
            port: u16::default(),
            dbname: String::default(),
            username: String::default(),
            password: Secret::default(),
            pool_size: default_pool_size(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

impl DatabaseSettings {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?application_name=observability",
            encode(&self.username),
            encode(self.password.peek()),
            self.host,
            self.port,
            self.dbname,
        )
    }

    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database host must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.dbname.is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database dbname must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.username.is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database username must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.password.peek().is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database password must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.port == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database port must be set".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.pool_size == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database pool_size must be greater than zero".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.connection_timeout == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database connection_timeout must be greater than zero".into(),
            ))
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8085,
            workers: 1,
        }
    }
}

impl Server {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "server host must not be empty".into(),
            ))
        })
    }
}

impl Settings<SecuredSecret> {
    pub fn new() -> Result<Self, errors::ConfigurationError> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(
        explicit_config_path: Option<PathBuf>,
    ) -> Result<Self, errors::ConfigurationError> {
        let environment = env::which();
        let config_path = explicit_config_path
            .unwrap_or_else(|| router_env::Config::get_config_directory().join(CONFIG_FILE_NAME));

        let config = router_env::Config::builder(&environment.to_string())?
            .add_source(File::from(config_path).required(false))
            .add_source(
                Environment::with_prefix("OBSERVABILITY")
                    .try_parsing(true)
                    .separator("__"),
            )
            .build()?;

        #[allow(clippy::print_stderr)]
        serde_path_to_error::deserialize(config).map_err(|error| {
            logger::error!(%error, "Unable to deserialize application configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            errors::ConfigurationError::from(error.into_inner())
        })
    }

    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        self.server.validate()?;
        self.auth.get_inner().validate()?;
        self.database.get_inner().validate()?;
        self.chat.get_inner().validate()?;
        self.email.validate()?;
        self.lifecycle.validate()?;
        self.instances.validate()?;
        self.mappers.validate()?;
        self.secrets_management
            .validate()
            .map_err(|error| errors::ConfigurationError::ConfigParsingError(error.into()))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn chat_with_ids(ids: &[&str]) -> ChatSettings {
        ChatSettings {
            destinations: ids
                .iter()
                .map(|id| ((*id).to_owned(), ChatDestination::Log))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_usable_destination_id_is_accepted() {
        chat_with_ids(&["sr_alerts", "zero-volume", "oncall2"])
            .validate()
            .unwrap();
    }

    #[test]
    fn ids_that_cannot_round_trip_through_the_environment_are_rejected() {
        for id in ["SR_ALERTS", "sr__alerts", ""] {
            assert!(
                chat_with_ids(&[id]).validate().is_err(),
                "`{id}` should be rejected"
            );
        }
    }

    #[test]
    fn a_zero_upload_cap_is_rejected() {
        let settings = ChatSettings {
            max_upload_bytes: 0,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn a_destination_is_selected_by_its_type_tag() {
        let destinations: HashMap<String, ChatDestination> =
            serde_json::from_value(serde_json::json!({
                "sr_alerts": { "type": "xyne", "app_jwt": "jwt", "channel": "C123" },
                "escalation": { "type": "slack", "bot_token": "xoxb-1", "channel": "C456" },
                "smoke": { "type": "log" },
            }))
            .unwrap();

        assert!(matches!(
            destinations.get("sr_alerts"),
            Some(ChatDestination::Xyne(_))
        ));
        assert!(matches!(
            destinations.get("escalation"),
            Some(ChatDestination::Slack(_))
        ));
        assert!(matches!(
            destinations.get("smoke"),
            Some(ChatDestination::Log)
        ));
    }

    fn database() -> DatabaseSettings {
        DatabaseSettings {
            host: "localhost".to_string(),
            port: 5432,
            dbname: "observability".to_string(),
            username: "alerts_app".to_string(),
            password: Secret::new("secret".to_string()),
            pool_size: 5,
            connection_timeout: 10,
        }
    }

    #[test]
    fn an_incomplete_database_configuration_fails_the_boot() {
        let cases: [(&str, DatabaseSettings); 5] = [
            (
                "host",
                DatabaseSettings {
                    host: String::new(),
                    ..database()
                },
            ),
            (
                "dbname",
                DatabaseSettings {
                    dbname: String::new(),
                    ..database()
                },
            ),
            (
                "username",
                DatabaseSettings {
                    username: String::new(),
                    ..database()
                },
            ),
            (
                "password",
                DatabaseSettings {
                    password: Secret::new(String::new()),
                    ..database()
                },
            ),
            (
                "port",
                DatabaseSettings {
                    port: 0,
                    ..database()
                },
            ),
        ];

        for (field, settings) in cases {
            assert!(
                settings.validate().is_err(),
                "an absent `{field}` should be rejected"
            );
        }
    }

    #[test]
    fn a_zero_pool_size_fails_the_boot() {
        assert!(DatabaseSettings {
            pool_size: 0,
            ..database()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn a_complete_database_configuration_is_accepted() {
        assert!(database().validate().is_ok());
    }

    #[test]
    fn the_connection_string_names_the_application() {
        assert!(database()
            .database_url()
            .contains("application_name=observability"));
    }

    #[test]
    fn the_database_password_is_not_in_the_debug_output() {
        let rendered = format!("{:?}", database());
        assert!(
            !rendered.contains("secret"),
            "password leaked into Debug: {rendered}"
        );
    }
}
