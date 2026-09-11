//! Application configuration.
//!
//! Read from `config/observability.toml` (override with `-f`), every value overridable by an
//! `OBSERVABILITY__`-prefixed environment variable using `__` to separate levels.
//!
//! [`Settings`] is generic over [`SecretState`]: deserialized as `Settings<SecuredSecret>` and
//! transitioned to `Settings<RawSecret>` at boot once KMS handles resolve, so "did we remember to
//! decrypt this?" is answered by the type checker. See [`crate::secrets_transformers`].

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

/// The default configuration file name, looked up inside the config directory.
const CONFIG_FILE_NAME: &str = "observability.toml";

/// Command line arguments accepted by the standalone binary.
#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/observability.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

/// The whole configuration of the service.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings<S: SecretState> {
    /// Listener configuration. Meaningful only in standalone mode — when the crate is mounted in
    /// the router, the router owns the listener and this section is ignored.
    pub server: Server,
    /// Logging and telemetry.
    pub log: Log,
    /// Credentials guarding this service's routes.
    pub auth: SecretStateContainer<AuthSettings, S>,
    /// The observability database. Its schema lives in `diesel_models::observability`.
    pub database: SecretStateContainer<DatabaseSettings, S>,
    /// How secret values in this file are resolved at boot.
    pub secrets_management: SecretsManagementConfig,
    /// Outbound HTTP proxy. A deployment fact rather than a property of any destination, which is
    /// why it sits here and is handed to every chat client rather than repeated per destination.
    pub proxy: Proxy,
    /// Chat destinations this service can deliver to.
    pub chat: SecretStateContainer<ChatSettings, S>,
    /// Email destinations this service can deliver to.
    pub email: EmailSettings,
}

const DEFAULT_MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

fn default_max_upload_bytes() -> usize {
    DEFAULT_MAX_UPLOAD_BYTES
}

/// Chat destinations, keyed by the id a request names.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ChatSettings {
    /// Every chat destination, by id.
    ///
    /// Ids from the environment are lowercased and cannot contain `__`: the `config` crate
    /// lowercases every env key before splitting and `__` is the level separator, so no spelling
    /// yields `SR_ALERTS`. [`ChatSettings::validate`] rejects such an id at boot rather than
    /// leaving a lookup to miss silently.
    pub destinations: HashMap<String, ChatDestination>,

    /// Maximum multipart body bytes accepted by the upload route.
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

/// One chat destination, tagged by the kind of backend it talks to.
///
/// Xyne and Slack are the same protocol with a different base URL and credential, so they are two
/// variants over one client rather than two integrations.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatDestination {
    /// A Xyne channel.
    Xyne(XyneConfig),
    /// A Slack channel.
    Slack(SlackConfig),
    /// Accepts messages and delivers nothing.
    ///
    /// For exercising the path before credentials exist. A destination *type* rather than a flag
    /// on a real destination, so the delivery path never has to ask whether it is pretending.
    Log,
}

/// Email destinations and the transport that serves them.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EmailSettings {
    /// The transport, shared by every destination: which backend, and who mail comes from.
    ///
    /// `external_services`' own type, reused wholesale so the SES / SMTP / no-email selection and
    /// its validation cannot drift from the router's. A few of its fields are unused here and
    /// default.
    ///
    /// Defaults to `NO_EMAIL_CLIENT`, which accepts and logs - the off switch, and what a
    /// deployment runs before SES credentials exist.
    #[serde(flatten)]
    pub client: EmailClientSettings,

    /// Every email destination, by id. Same id constraints as [`ChatSettings::destinations`].
    pub destinations: HashMap<String, EmailDestination>,
}

/// One email destination.
#[derive(Debug, Deserialize, Clone)]
pub struct EmailDestination {
    /// Where the alert goes.
    ///
    /// A single address: `EmailClient::send_email` accepts one. Reaching three people is three
    /// destinations today; widening to a list later changes no caller, since a request only ever
    /// names an id.
    pub to: pii::Email,
}

/// Ids are addressed by callers and set from the environment, so they must survive both. `config`
/// lowercases environment keys and splits on `__`; an id that would come back different is
/// rejected at boot rather than silently failing to match at lookup time.
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
    /// Reject destination ids that cannot be set from the environment.
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
    /// Reject destination ids that cannot be set from the environment, an unusable transport, and
    /// a destination with no address.
    ///
    /// The transport check is `external_services`' own, and runs only when destinations exist: a
    /// deployment with none would otherwise need a verified SES sender to be deployable at all.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_destination_ids(&self.destinations, "email")?;

        if self.destinations.is_empty() {
            return Ok(());
        }

        self.client
            .validate()
            .map_err(|error| errors::ConfigurationError::ConfigParsingError(error.to_owned()))?;

        // `EmailSettings::validate` checks only the backend-specific section — SES's role ARN,
        // SMTP's host — and never the shared sender. It defaults to an empty `pii::Email`, which
        // SMTP then fails on while building the `From` mailbox and SES submits as an empty sender.
        // Without this, a missing `sender_email` boots cleanly and turns every alert into a 502.
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

/// Credentials guarding this service's routes.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AuthSettings {
    /// The key callers must supply in the `X-Internal-Api-Key` header.
    ///
    /// Deliberately *not* the router's `secrets.admin_api_key`: that would let any admin
    /// credential send alerts, and tie this service's rotation to the router's.
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
    /// Reject an absent internal API key.
    ///
    /// There is deliberately no way to disable authentication. An empty key does not mean "open"
    /// — it means the service refuses to start.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.internal_api_key.peek().is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "auth internal_api_key must not be empty".into(),
            ))
        })
    }
}

/// Default pool size. Small on purpose: the service issues a handful of queries per alert run, and
/// the role it connects as carries a connection limit that the pool has to fit inside — alongside
/// the migration job, which needs connections of its own at deploy time.
const DEFAULT_POOL_SIZE: u32 = 5;

/// Default seconds to wait for a connection before giving up.
const DEFAULT_CONNECTION_TIMEOUT: u64 = 10;

fn default_pool_size() -> u32 {
    DEFAULT_POOL_SIZE
}

fn default_connection_timeout() -> u64 {
    DEFAULT_CONNECTION_TIMEOUT
}

/// The observability database.
///
/// Not `hyperswitch_db`: these tables live in their own database, so that alert state is not
/// written into a store the application owns.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DatabaseSettings {
    /// Hostname of the cluster.
    pub host: String,
    /// Port the cluster listens on.
    pub port: u16,
    /// The database within the cluster. Not the schema — everything lives in `public`.
    pub dbname: String,
    /// The role this service connects as. Deliberately not the migration role: this one holds no
    /// DDL rights, so a running service cannot alter the schema it reads.
    pub username: String,
    /// The role's password. Under a KMS backend this is ciphertext until [`crate::secrets_transformers`]
    /// resolves it.
    pub password: Secret<String>,
    /// Connections held open.
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Seconds to wait for a connection from the pool before giving up.
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
}

/// Percent-encode a connection-string component.
///
/// A generated password routinely contains `/`, `@`, `?` or `#`, each of which ends a field in a
/// URI - libpq would read the host, database and query parameters from the wrong side of it.
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

/// Hand-written so that `Default` and the `#[serde(default = ...)]` attributes agree.
///
/// Deriving it would set `pool_size` and `connection_timeout` to zero, which the serde defaults
/// exist to prevent and which bb8 asserts against — the same reason `ChatSettings` writes its own.
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
    /// The connection string.
    ///
    /// Not [`common_utils::DbConnectionParams::get_database_url`], which sets `application_name` to
    /// the schema for the router's per-tenant `search_path`. Naming the application is what lets
    /// `pg_stat_activity` attribute connections on a shared cluster.
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

    /// Reject a configuration that cannot connect.
    ///
    /// Checked before anything is dialled, so a missing value fails the boot rather than the first
    /// query. Pool size is deliberately not checked against the role's connection limit: this
    /// service cannot see that limit, and guessing would turn a database-side change into a
    /// mystery boot failure.
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

        // bb8 asserts both of these are non-zero when the pool is built, and an assert names
        // neither the key nor the file it came from. Rejecting them here turns a panic at boot
        // into a message that says which setting is wrong.
        common_utils::fp_utils::when(self.connection_timeout == 0, || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "database connection_timeout must be greater than zero".into(),
            ))
        })
    }
}

/// Listener configuration for the standalone binary.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    /// Port to bind to.
    pub port: u16,
    /// Number of actix workers.
    pub workers: usize,
    /// Host to bind to.
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
    /// Reject an empty bind host.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(errors::ConfigurationError::ConfigParsingError(
                "server host must not be empty".into(),
            ))
        })
    }
}

impl Settings<SecuredSecret> {
    /// Read configuration from the default location.
    pub fn new() -> Result<Self, errors::ConfigurationError> {
        Self::with_config_path(None)
    }

    /// Read configuration, optionally from an explicit path.
    ///
    /// Values are resolved in the following priority order (1 being least priority):
    ///
    /// 1. Defaults from the implementation of the `Default` trait.
    /// 2. Values from the config file — `config/observability.toml` unless overridden by `-f`. The
    ///    config directory itself can be moved with the `CONFIG_DIR` environment variable.
    /// 3. Environment variables prefixed with `OBSERVABILITY` and each level separated by double
    ///    underscores, e.g. `OBSERVABILITY__AUTH__INTERNAL_API_KEY`.
    ///
    /// Unlike `drainer`, this service reads a file dedicated to it rather than the shared
    /// per-environment config: it has no runnable defaults to fall back on, since there is no
    /// sensible default for an API key.
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

        // The logger may not yet be initialized when constructing the application configuration
        #[allow(clippy::print_stderr)]
        serde_path_to_error::deserialize(config).map_err(|error| {
            logger::error!(%error, "Unable to deserialize application configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            errors::ConfigurationError::from(error.into_inner())
        })
    }

    /// Reject an unusable configuration.
    ///
    /// Called before anything is bound or connected, so that a misconfiguration surfaces as a
    /// failure to start rather than as a failure on the first alert — by which time whoever
    /// deployed it has stopped watching.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        self.server.validate()?;
        self.auth.get_inner().validate()?;
        self.database.get_inner().validate()?;
        self.chat.get_inner().validate()?;
        self.email.validate()?;
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

    /// Each of these is a lookup that would silently miss when the id came from the environment,
    /// because `config` lowercases keys and splits on `__`.
    #[test]
    fn ids_that_cannot_round_trip_through_the_environment_are_rejected() {
        for id in ["SR_ALERTS", "sr__alerts", ""] {
            assert!(
                chat_with_ids(&[id]).validate().is_err(),
                "`{id}` should be rejected"
            );
        }
    }

    /// The tag is what makes Xyne and Slack two variants of one client rather than two
    /// integrations, and `log` has to sit in the same enum or the delivery path grows a branch.
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

    /// Every one of these fails at connection time rather than at boot, and a connection failure
    /// names a host and a role — so it reads as a wrong credential rather than an absent one, at
    /// whatever hour the first alert run happens to be.
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

    /// A pool of zero connections never serves a query and never errors either: every caller waits
    /// for the connection timeout and gets a timeout, which looks like a slow database.
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

    /// On a shared cluster `pg_stat_activity` has to be able to answer whose connections are
    /// whose. Without this the answer is the schema name, which is `public` for every tenant.
    #[test]
    fn the_connection_string_names_the_application() {
        assert!(database()
            .database_url()
            .contains("application_name=observability"));
    }

    /// The password reaches the log the moment someone debugs the configuration, unless the type
    /// prevents it. A hand-written `Debug` would do the same job until a field is added and
    /// somebody forgets.
    #[test]
    fn the_database_password_is_not_in_the_debug_output() {
        let rendered = format!("{:?}", database());
        assert!(
            !rendered.contains("secret"),
            "password leaked into Debug: {rendered}"
        );
    }
}
