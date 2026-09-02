//! Application configuration.
//!
//! Read from `config/alerts.toml` (override with `-f`), with every value overridable by an
//! `ALERTS__`-prefixed environment variable using `__` to separate levels.
//!
//! [`Settings`] is generic over [`SecretState`]: it is deserialized as `Settings<SecuredSecret>`,
//! where secret values may be KMS handles, and transitions to `Settings<RawSecret>` at boot once
//! those handles have been resolved. See [`crate::secrets_transformers`]. Only a `RawSecret`
//! configuration can be used to serve requests, so "did we remember to decrypt this?" is answered
//! by the type checker rather than by review.

use std::{collections::HashMap, path::PathBuf};

use common_utils::{ext_traits::ConfigExt, pii};
use config::{Environment, File};
use external_services::{
    chat_service::{slack::SlackConfig, xyne::XyneConfig},
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
const CONFIG_FILE_NAME: &str = "alerts.toml";

/// Command line arguments accepted by the standalone binary.
#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/alerts.toml" if this option isn't specified.
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

/// Chat destinations, keyed by the id a request names.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ChatSettings {
    /// Every chat destination, by id.
    ///
    /// **Ids arriving from the environment are lowercased and cannot contain `__`.** The `config`
    /// crate lowercases every environment key before splitting it (`config-0.14.1/src/env.rs`),
    /// and `__` is the level separator, so `ALERTS__CHAT__DESTINATIONS__SR_ALERTS__CHANNEL` sets
    /// `chat.destinations.sr_alerts.channel` and there is no spelling that yields `SR_ALERTS` or
    /// `sr__alerts`. [`ChatSettings::validate`] rejects an id that cannot survive the round trip,
    /// so this is a boot failure rather than a lookup that mysteriously misses.
    pub destinations: HashMap<String, ChatDestination>,
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

/// Email destinations, keyed by the id a request names.
///
/// No transport configuration yet: the email client is chosen in the email transport ticket, which
/// owns how `alerts` reaches `external_services::email`. Until then every destination here
/// resolves to a log destination.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EmailSettings {
    /// Every email destination, by id. Same id constraints as [`ChatSettings::destinations`].
    pub destinations: HashMap<String, EmailDestination>,
}

/// One email destination.
#[derive(Debug, Deserialize, Clone)]
pub struct EmailDestination {
    /// Where the alert goes.
    ///
    /// A single address, because `EmailClient::send_email` accepts one and both backends build a
    /// single-recipient message. Reaching three people is three destinations today; when
    /// a follow-up ticket lands this widens to a list and no caller changes, since a request
    /// only ever names an id.
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
        validate_destination_ids(&self.destinations, "chat")
    }
}

impl EmailSettings {
    /// Reject destination ids that cannot be set from the environment.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_destination_ids(&self.destinations, "email")
    }
}

/// Credentials guarding this service's routes.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AuthSettings {
    /// The key callers must supply in the `X-Internal-Api-Key` header.
    ///
    /// Deliberately *not* the router's `secrets.admin_api_key`: reusing that would mean anyone
    /// holding admin credentials could send alerts, and would tie this service's rotation
    /// schedule to the router's.
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
    /// 2. Values from the config file — `config/alerts.toml` unless overridden by `-f`. The
    ///    config directory itself can be moved with the `CONFIG_DIR` environment variable.
    /// 3. Environment variables prefixed with `ALERTS` and each level separated by double
    ///    underscores, e.g. `ALERTS__AUTH__INTERNAL_API_KEY`.
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
                Environment::with_prefix("ALERTS")
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
}
