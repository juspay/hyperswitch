//!
//! Logger-specific config.
//!
//! Looking for config files algorithm first tries to deduce type of environment ( `Development`/`Sandbox`/`Production` ) from environment variable `RUN_ENV`.
//! It uses type of environment to deduce which config to load.
//! Default config is `/config/Development.toml`.
//! Default type of environment is `Development`.
//! It falls back to defaults defined in file "defaults.toml" in src if no config file found or it does not have some key value pairs.
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
#[derive(Debug, Deserialize, Clone)]
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
pub struct Level(tracing::Level);

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
#[derive(Debug, Deserialize, Clone)]
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
        Ok(config::Config::builder()
            // Here should be `set_override` not `set_default`.
            // "env" can't be altered by config field.
            // Should be single source of truth.
            .set_override("env", environment)?
            .add_source(config::File::from_str(
                // Plan on handling with the changes in crates/router
                // FIXME: embedding of textual file into bin files has several disadvantages
                // 1. larger bin file
                // 2. slower initialization of program
                // 3. too late ( run-time ) information about broken toml file
                // Consider embedding all defaults into code.
                // Example: https://github.com/instrumentisto/medea/blob/medea-0.2.0/src/conf/mod.rs#L60-L102
                include_str!("defaults.toml"),
                config::FileFormat::Toml,
            )))
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
