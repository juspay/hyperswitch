use std::path::PathBuf;

use config::{Environment, File, FileFormat};
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::Deserialize;
use structopt::StructOpt;

use crate::{
    core::errors::{BachError, BachResult},
    env::{self, logger, Env},
};

#[derive(StructOpt, Default)]
#[structopt(version = router_env::version!())]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[structopt(short = "f", long, parse(from_os_str))]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
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
    pub scheduler: Option<SchedulerSettings>,
    #[cfg(feature = "kv_store")]
    pub drainer: DrainerSettings,
    pub jwekey: Jwekey,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Keys {
    #[cfg(feature = "kms")]
    pub aws_key_id: String,
    #[cfg(feature = "kms")]
    pub aws_region: String,
    pub temp_card_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Locker {
    pub host: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub port: u16,
    pub host: String,
    pub request_body_limit: usize,
    pub base_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    // TODO(kos): Consider using `Box<str>` here (and in similar places), as we
    // don't need any `String` mutation capabilities for this field
    // (after parsing it will only be read, but never mutated).
    pub username: String,
    // FIXME(kos): Any kind of secrets, we control, should be wrapped into
    // Secret, which will:
    // 1. Protect any accidental secrets leaks to logs, traces or
    //     any other `Debug` prints.
    // 2. Correctly zeroize the memory of secrets on `Drop` to avoid
    //     accidental secrets leaks when process is crashed and core
    //     dumped (by accessing the core dump file), process memory
    //     is swapped, etc.
    // There is a similar, extended, but custom made functionality
    // in the `masking` crate. So, the team is aware of the problem.
    // But why then the `masking::Secret` is not used here and in
    // similar places?
    pub password: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub adyen: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub checkout: ConnectorParams,
    pub stripe: ConnectorParams,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorParams {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
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
}

impl Settings {
    pub fn new() -> BachResult<Self> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(config_path: Option<PathBuf>) -> BachResult<Self> {
        let environment = env::which();
        let config_path = router_env::Config::config_path(&environment.to_string(), config_path);

        // println!("config_path : {:?}", config_path);
        // println!("current_dir : {:?}", std::env::current_dir());

        let config = router_env::Config::builder(&environment.to_string())?
            // FIXME: consider embedding of textual file into bin files has several disadvantages
            // 1. larger bin file
            // 2. slower initialization of program
            // 3. too late ( run-time ) information about broken toml file
            // Consider embedding all defaults into code.
            // Example: https://github.com/instrumentisto/medea/blob/medea-0.2.0/src/conf/mod.rs#L60-L102
            .add_source(File::from_str(
                include_str!("defaults.toml"),
                FileFormat::Toml,
            ))
            // FIXME(kos): `.required(true)` seems to be unnecessarily strict
            // here. It's totally common scenario to run a process
            // and provide it with environment variables only,
            // without any config files. So, requiring a config file
            // always to exist will be only a stumble factor for
            // users.
            .add_source(File::from(config_path).required(true))
            .add_source(
                Environment::with_prefix("ROUTER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("redis.cluster_urls"),
            )
            .build()?;

        config.try_deserialize().map_err(|e| {
            // FIXME(kos): We can improve it.
            // Usually, the logging system is initialized after the
            // config parsing is done, as the config itself would
            // likely have settings for logging setup. That's why
            // doing logging here before the parsed config is
            // returned is kinda strange thing.
            // We can simply return the error.
            // Let the caller decide what to do with this
            // error. Hardcoded printing side effects here may be
            // undesired by the caller.
            logger::error!("Unable to source config file");
            eprintln!("Unable to source config file");
            BachError::from(e)
        })
    }
}
