use std::{path::PathBuf, sync::Arc};

use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
use external_services::managers::{
    encryption_management::EncryptionManagementConfig, secrets_management::SecretsManagementConfig,
};
use hyperswitch_interfaces::{
    encryption_interface::EncryptionManagementInterface,
    secrets_interface::secret_state::{
        RawSecret, SecretState, SecretStateContainer, SecuredSecret,
    },
};
use masking::Secret;
use redis_interface as redis;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use router_env::{env, logger};
use serde::Deserialize;

use crate::{errors, secrets_decryption};

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct AppState {
    pub conf: Arc<Settings<RawSecret>>,
    pub encryption_client: Box<dyn EncryptionManagementInterface>,
}

impl AppState {
    /// # Panics
    ///
    /// Panics if secret or encryption management client cannot be initiated
    pub async fn new(conf: Settings<SecuredSecret>) -> Self {
        #[allow(clippy::expect_used)]
        let secret_management_client = conf
            .secrets_management
            .get_secret_management_client()
            .await
            .expect("Failed to create secret management client");

        let conf = secrets_decryption::kms_decryption(conf, secret_management_client).await;

        #[allow(clippy::expect_used)]
        let encryption_client = conf
            .encryption_management
            .get_encryption_management_client()
            .await
            .expect("Failed to create encryption management client");

        Self {
            conf: Arc::new(conf),
            encryption_client,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings<S: SecretState> {
    pub server: Server,
    pub master_database: SecretStateContainer<Database, S>,
    pub redis: redis::RedisSettings,
    pub log: Log,
    pub drainer: DrainerSettings,
    pub encryption_management: EncryptionManagementConfig,
    pub secrets_management: SecretsManagementConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
}

impl Server {
    pub fn validate(&self) -> Result<(), errors::DrainerError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "server host must not be empty".into(),
            ))
        })
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new().into(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
        }
    }
}

impl Default for DrainerSettings {
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

impl Default for Server {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 1,
        }
    }
}

impl Database {
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
    fn validate(&self) -> Result<(), errors::DrainerError> {
        common_utils::fp_utils::when(self.stream_name.is_default_or_empty(), || {
            Err(errors::DrainerError::ConfigParsingError(
                "drainer stream name must not be empty".into(),
            ))
        })
    }
}

impl Settings<SecuredSecret> {
    pub fn new() -> Result<Self, errors::DrainerError> {
        Self::with_config_path(None)
    }

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

    pub fn validate(&self) -> Result<(), errors::DrainerError> {
        self.server.validate()?;
        self.master_database.get_inner().validate()?;
        self.redis.validate().map_err(|error| {
            println!("{error}");
            errors::DrainerError::ConfigParsingError("invalid Redis configuration".into())
        })?;
        self.drainer.validate()?;
        self.secrets_management.validate().map_err(|error| {
            println!("{error}");
            errors::DrainerError::ConfigParsingError(
                "invalid secrets management configuration".into(),
            )
        })?;

        self.encryption_management.validate().map_err(|error| {
            println!("{error}");
            errors::DrainerError::ConfigParsingError(
                "invalid encryption management configuration".into(),
            )
        })?;

        Ok(())
    }
}
