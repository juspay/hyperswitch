use std::path::PathBuf;

use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::Deserialize;

use crate::{
    core::errors::{ApplicationError, ApplicationResult},
    env::{self, logger, Env},
};

#[derive(clap::Parser, Default)]
#[command(version = router_env::version!())]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(clap::Parser)]
pub enum Subcommand {
    #[cfg(feature = "openapi")]
    /// Generate the OpenAPI specification file from code.
    GenerateOpenapiSpec,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings {
    pub server: Server,
    pub proxy: Proxy,
    pub env: Env,
    pub master_database: Database,
    #[cfg(feature = "olap")]
    pub replica_database: Database,
    pub redis: RedisSettings,
    pub log: Log,
    pub secrets: Secrets,
    pub locker: Locker,
    pub connectors: Connectors,
    pub refund: Refund,
    pub eph_key: EphemeralConfig,
    pub scheduler: Option<SchedulerSettings>,
    #[cfg(feature = "kv_store")]
    pub drainer: DrainerSettings,
    pub jwekey: Jwekey,
    pub webhooks: WebhooksSettings,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Secrets {
    pub jwt_secret: String,
    pub admin_api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Locker {
    pub host: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Refund {
    pub max_attempts: usize,
    pub max_age: i64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct EphemeralConfig {
    pub validity: i64,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Jwekey {
    #[cfg(feature = "kms")]
    pub aws_key_id: String,
    #[cfg(feature = "kms")]
    pub aws_region: String,
    pub locker_key_identifier1: String,
    pub locker_key_identifier2: String,
    pub locker_encryption_key1: String,
    pub locker_encryption_key2: String,
    pub locker_decryption_key1: String,
    pub locker_decryption_key2: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
    pub request_body_limit: usize,
    pub base_url: String,
    pub shutdown_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Database {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SupportedConnectors {
    pub wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub adyen: ConnectorParams,
    pub applepay: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub braintree: ConnectorParams,
    pub checkout: ConnectorParams,
    pub cybersource: ConnectorParams,
    pub dlocal: ConnectorParams,
    pub fiserv: ConnectorParams,
    pub globalpay: ConnectorParams,
    pub klarna: ConnectorParams,
    pub payu: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub shift4: ConnectorParams,
    pub stripe: ConnectorParams,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,

    // Keep this field separate from the remaining fields
    pub supported: SupportedConnectors,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConnectorParams {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSettings {
    pub stream: String,
    pub producer: ProducerSettings,
    pub consumer: ConsumerSettings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProducerSettings {
    pub upper_fetch_limit: i64,
    pub lower_fetch_limit: i64,

    pub lock_key: String,
    pub lock_ttl: i64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ConsumerSettings {
    pub disabled: bool,
    pub consumer_group: String,
}

#[cfg(feature = "kv_store")]
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
    pub shutdown_interval: u32, // in milliseconds
    pub loop_interval: u32,     // in milliseconds
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WebhooksSettings {
    pub outgoing_enabled: bool,
}

impl Settings {
    pub fn new() -> ApplicationResult<Self> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(config_path: Option<PathBuf>) -> ApplicationResult<Self> {
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

        let environment = env::which();
        let config_path = router_env::Config::config_path(&environment.to_string(), config_path);

        let config = router_env::Config::builder(&environment.to_string())?
            .add_source(File::from(config_path).required(true))
            .add_source(
                Environment::with_prefix("ROUTER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("redis.cluster_urls")
                    .with_list_parse_key("connectors.supported.wallets"),
            )
            .build()?;

        serde_path_to_error::deserialize(config).map_err(|error| {
            logger::error!(%error, "Unable to deserialize application configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            ApplicationError::from(error.into_inner())
        })
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        self.server.validate()?;
        self.master_database.validate()?;
        #[cfg(feature = "olap")]
        self.replica_database.validate()?;
        self.redis.validate().map_err(|error| {
            println!("{error}");
            ApplicationError::InvalidConfigurationValueError("Redis configuration".into())
        })?;
        if self.log.file.enabled {
            if self.log.file.file_name.is_default_or_empty() {
                return Err(ApplicationError::InvalidConfigurationValueError(
                    "log file name must not be empty".into(),
                ));
            }

            if self.log.file.path.is_default_or_empty() {
                return Err(ApplicationError::InvalidConfigurationValueError(
                    "log directory path must not be empty".into(),
                ));
            }
        }
        self.secrets.validate()?;
        self.locker.validate()?;
        self.connectors.validate()?;

        self.scheduler
            .as_ref()
            .map(|scheduler_settings| scheduler_settings.validate())
            .transpose()?;
        #[cfg(feature = "kv_store")]
        self.drainer.validate()?;
        self.jwekey.validate()?;

        Ok(())
    }
}
