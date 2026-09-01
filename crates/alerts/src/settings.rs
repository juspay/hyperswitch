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

use std::path::PathBuf;

use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
use external_services::managers::secrets_management::SecretsManagementConfig;
use hyperswitch_interfaces::secrets_interface::secret_state::{
    SecretState, SecretStateContainer, SecuredSecret,
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
        self.secrets_management
            .validate()
            .map_err(|error| errors::ConfigurationError::ConfigParsingError(error.into()))?;
        Ok(())
    }
}
