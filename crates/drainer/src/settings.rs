use std::path::PathBuf;

use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
#[cfg(feature = "hashicorp-vault")]
use external_services::hashicorp_vault;
#[cfg(feature = "kms")]
use external_services::kms;
use redis_interface as redis;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use router_env::{env, logger};
use serde::Deserialize;

use crate::errors;

#[cfg(feature = "kms")]
pub type Password = kms::KmsValue;
#[cfg(not(feature = "kms"))]
pub type Password = masking::Secret<String>;

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings {
    pub master_database: Database,
    pub redis: redis::RedisSettings,
    pub log: Log,
    pub drainer: DrainerSettings,
    #[cfg(feature = "kms")]
    pub kms: kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    pub hc_vault: hashicorp_vault::HashiCorpVaultConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Database {
    pub username: String,
    pub password: Password,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
    pub shutdown_interval: u32, // in milliseconds
    pub loop_interval: u32,     // in milliseconds
}

impl Default for Database {
        /// Returns a new instance of the struct with default values for the username, password, host, port, dbname, pool size, and connection timeout.
    fn default() -> Self {
        Self {
            username: String::new(),
            password: Password::default(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
        }
    }
}

impl Default for DrainerSettings {
        /// Creates a new instance of the structure with default values.
    fn default() -> Self {
        Self {
            stream_name: "DRAINER_STREAM".into(),
            num_partitions: 64,
            max_read_count: 100,
            shutdown_interval: 1000, // in milliseconds
            loop_interval: 100,      // in milliseconds
        }
    }
}

impl Database {
        /// Validates the database configuration by checking if the host, dbname, username, and password are not empty or default values.
    fn validate(&self) -> Result<(), errors::DrainerError> {
        use common_utils::fp_utils::when;

        when(self.host.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "database host must not be empty".into(),
            ))
        })?;

        when(self.dbname.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "database name must not be empty".into(),
            ))
        })?;

        when(self.username.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "database user username must not be empty".into(),
            ))
        })?;

        when(self.password.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "database user password must not be empty".into(),
            ))
        })
    }
}

impl DrainerSettings {
        /// Validates the stream name and returns a Result indicating success or a DrainerError if the stream name is empty.
    fn validate(&self) -> Result<(), errors::DrainerError> {
        common_utils::fp_utils::when(self.stream_name.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "drainer stream name must not be empty".into(),
            ))
        })
    }
}

impl Settings {
        /// Creates a new instance of the Drainer with a default configuration path. 
    ///
    /// # Returns
    /// 
    /// A Result containing the new instance of Drainer if successful, or a DrainerError if an error occurs.
    pub fn new() -> Result<Self, errors::DrainerError> {
        Self::with_config_path(None)
    }

    /// Retrieves the application configuration values based on the specified priority order. The method first checks for defaults from the implementation of the `Default` trait, then looks for values from a config file based on the environment specified by the `RUN_ENV` environment variable (which can be `development`, `sandbox` or `production`). If no `RUN_ENV` is specified, the method reads from the `/config/development.toml` file. Finally, the method checks for environment variables prefixed with `DRAINER` and each level separated by double underscores. Values in the config file override the defaults, and the values set using environment variables override both the defaults and the config file values.
    pub fn with_config_path(config_path: Option<PathBuf>) -> Result<Self, errors::DrainerError> {
        // Configuration values are picked up in the following priority order (1 being least
        // priority):
        // 1. Defaults from the implementation of the `Default` trait.
        // 2. Values from config file. The config file accessed depends on the environment
        //    specified by the `RUN_ENV` environment variable. `RUN_ENV` can be one of
        //    `development`, `sandbox` or `production`. If nothing is specified for `RUN_ENV`,
        //    `/config/development.toml` file is read.
        // 3. Environment variables prefixed with `DRAINER` and each level separated by double
        //    underscores.
        //
        // Values in config file override the defaults in `Default` trait, and the values set using
        // environment variables override both the defaults and the config file values.

        let environment = env::which();
        let config_path = router_env::Config::config_path(&environment.to_string(), config_path);

        let config = router_env::Config::builder(&environment.to_string())?
            .add_source(File::from(config_path).required(false))
            .add_source(
                Environment::with_prefix("DRAINER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("redis.cluster_urls"),
            )
            .build()?;

        serde_path_to_error::deserialize(config).map_err(|error| {
            logger::error!(%error, "Unable to deserialize application configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            errors::DrainerError::from(error.into_inner())
        })
    }

        /// Validates the configuration of the current instance by ensuring that the master database, Redis, and the drainer are all properly configured and reachable.
    /// 
    /// # Returns
    /// 
    /// Returns a Result indicating success or a `DrainerError` if any validation fails.
    /// 
    /// # Errors
    /// 
    /// Returns a `DrainerError::ConfigParsingError` if the Redis configuration is invalid, otherwise returns any errors encountered during validation of the master database or the drainer.
    pub fn validate(&self) -> Result<(), errors::DrainerError> {
        self.master_database.validate()?;
        self.redis.validate().map_err(|error| {
            println!("{error}");
            errors::DrainerError::ConfigParsingError("invalid Redis configuration".into())
        })?;
        self.drainer.validate()?;

        Ok(())
    }
}
