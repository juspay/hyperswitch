use std::path::PathBuf;

use config::{Environment, File, FileFormat};
use redis_interface as redis;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use router_env::{env, logger};
use serde::Deserialize;
use structopt::StructOpt;

use crate::errors;

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
    pub master_database: Database,
    pub redis: redis::RedisSettings,
    pub log: Log,
    pub drainer: DrainerSettings,
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

#[derive(Debug, Clone, Deserialize)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
}

impl Settings {
    pub fn new() -> Result<Self, errors::DrainerError> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(config_path: Option<PathBuf>) -> Result<Self, errors::DrainerError> {
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
            .add_source(File::from(config_path).required(true))
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
}
