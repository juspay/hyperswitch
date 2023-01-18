//!
//! Logger-specific config.
//!

use std::path::PathBuf;

use serde::Deserialize;

/// Directory of config toml files. Default is config
pub const CONFIG_DIR: &str = "CONFIG_DIR";

/// Config settings.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Logging to a file.
    pub log: Log,
}

/// Log config settings.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Log {
    /// Logging to a file.
    pub file: LogFile,
    /// Logging to a console.
    pub console: LogConsole,
    /// Telemetry / tracing.
    pub telemetry: LogTelemetry,
}

/// Logging to a file.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LogFile {
    /// Whether you want to store log in log files.
    pub enabled: bool,
    /// Where to store log files.
    pub path: String,
    /// Name of log file without suffix.
    pub file_name: String,
    // pub do_async: bool, // is not used
    /// What gets into log files.
    pub level: Level,
    // pub rotation: u16,
}

/// Describes the level of verbosity of a span or event.
#[derive(Debug, Clone)]
pub struct Level(pub(super) tracing::Level);

impl Level {
    /// Returns the most verbose [`tracing::Level`]
    pub fn into_level(&self) -> tracing::Level {
        self.0
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr as _;

        let s = String::deserialize(deserializer)?;
        tracing::Level::from_str(&s)
            .map(Level)
            .map_err(serde::de::Error::custom)
    }
}

/// Logging to a console.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LogConsole {
    /// Whether you want to see log in your terminal.
    pub enabled: bool,
    /// What you see in your terminal.
    pub level: Level,
    /// Log format
    #[serde(default)]
    pub log_format: LogFormat,
}

/// Telemetry / tracing.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LogTelemetry {
    /// Whether tracing/telemetry is enabled.
    pub enabled: bool,
    /// Sampling rate for traces
    pub sampling_rate: Option<f64>,
}

/// Telemetry / tracing.
#[derive(Default, Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Default pretty log format
    Default,
    /// JSON based structured logging
    #[default]
    Json,
}

impl Config {
    /// Default constructor.
    pub fn new() -> Result<Self, config::ConfigError> {
        Self::new_with_config_path(None)
    }

    /// Constructor expecting config path set explicitly.
    pub fn new_with_config_path(
        explicit_config_path: Option<PathBuf>,
    ) -> Result<Self, config::ConfigError> {
        // Configuration values are picked up in the following priority order (1 being least
        // priority):
        // 1. Defaults from the implementation of the `Default` trait.
        // 2. Values from config file. The config file accessed depends on the environment
        //    specified by the `RUN_ENV` environment variable. `RUN_ENV` can be one of
        //    `Development`, `Sandbox` or `Production`. If nothing is specified for `RUN_ENV`,
        //    `/config/Development.toml` file is read.
        // 3. Environment variables prefixed with `ROUTER` and each level separated by double
        //    underscores.
        //
        // Values in config file override the defaults in `Default` trait, and the values set using
        // environment variables override both the defaults and the config file values.

        let environment = crate::env::which();
        let config_path = Self::config_path(&environment.to_string(), explicit_config_path);

        let config = Self::builder(&environment.to_string())?
            .add_source(config::File::from(config_path).required(true))
            .add_source(config::Environment::with_prefix("ROUTER").separator("__"))
            .build()?;

        serde_path_to_error::deserialize(config).map_err(|error| {
            crate::error!(%error, "Unable to deserialize configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            error.into_inner()
        })
    }

    /// Construct config builder extending it by fall-back defaults and setting config file to load.
    pub fn builder(
        environment: &str,
    ) -> Result<config::ConfigBuilder<config::builder::DefaultState>, config::ConfigError> {
        config::Config::builder()
            // Here, it should be `set_override()` not `set_default()`.
            // "env" can't be altered by config field.
            // Should be single source of truth.
            .set_override("env", environment)
    }

    /// Config path.
    pub fn config_path(environment: &str, explicit_config_path: Option<PathBuf>) -> PathBuf {
        let mut config_path = PathBuf::new();
        if let Some(explicit_config_path_val) = explicit_config_path {
            config_path.push(explicit_config_path_val);
        } else {
            let config_directory = std::env::var(CONFIG_DIR).unwrap_or_else(|_| "config".into());
            let config_file_name = match environment {
                "Production" => "Production.toml",
                "Sandbox" => "Sandbox.toml",
                _ => "Development.toml",
            };

            config_path.push(crate::env::workspace_path());
            config_path.push(config_directory);
            config_path.push(config_file_name);
        }
        config_path
    }
}
