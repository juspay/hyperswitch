use std::path::PathBuf;

use config::{Environment, File};
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::Deserialize;
use structopt::StructOpt;

use crate::{
    core::errors::{ApplicationError, ApplicationResult},
    env::{self, logger, Env},
};

#[derive(StructOpt, Default)]
#[structopt(version = router_env::version!())]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[structopt(short = "f", long, parse(from_os_str))]
    pub config_path: Option<PathBuf>,

    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(StructOpt)]
pub enum Subcommand {
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
    pub keys: Keys, //remove this during refactoring
    pub locker: Locker,
    pub connectors: Connectors,
    pub refund: Refund,
    pub eph_key: EphemeralConfig,
    pub scheduler: Option<SchedulerSettings>,
    #[cfg(feature = "kv_store")]
    pub drainer: DrainerSettings,
    pub jwekey: Jwekey,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Keys {
    #[cfg(feature = "kms")]
    pub aws_key_id: String,
    #[cfg(feature = "kms")]
    pub aws_region: String,
    pub temp_card_key: String,
    pub jwt_secret: String,
    pub admin_api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Locker {
    pub host: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Refund {
    pub max_attempts: usize,
    pub max_age: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EphemeralConfig {
    pub validity: i64,
}

#[derive(Debug, Deserialize, Clone, Default)]
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
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
    pub request_body_limit: usize,
    pub base_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SupportedConnectors {
    pub wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub adyen: ConnectorParams,
    pub applepay: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub braintree: ConnectorParams,
    pub checkout: ConnectorParams,
    pub cybersource: ConnectorParams,
    pub klarna: ConnectorParams,
    pub shift4: ConnectorParams,
    pub stripe: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub supported: SupportedConnectors,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorParams {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSettings {
    pub stream: String,
    pub consumer_group: String,
    pub producer: ProducerSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProducerSettings {
    pub upper_fetch_limit: i64,
    pub lower_fetch_limit: i64,

    pub lock_key: String,
    pub lock_ttl: i64,
    pub batch_size: usize,
}

#[cfg(feature = "kv_store")]
#[derive(Debug, Clone, Deserialize)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
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
}
