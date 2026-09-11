use common_utils::DbConnectionParams;
use hyperswitch_domain_models::master_key::MasterKeyInterface;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{kv_router_store, DatabaseStore, MockDb, RouterStore};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    #[serde(alias = "pool_size")]
    pub max_pool_size: u32,
    pub connection_timeout: u64,
    pub queue_strategy: QueueStrategy,
    #[serde(alias = "min_idle", default = "default_min_idle_pool_size")]
    pub min_idle_pool_size: u32,
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime: u64,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
    /// Run a liveness query on every checkout from the pool.
    ///
    /// bb8 defaults this on, which costs one extra `SELECT 1` round trip per
    /// checkout. That is cheap against a local Postgres and expensive against
    /// Spanner through PGAdapter, where the probe carries the same per-RPC
    /// overhead as a real query, so it is worth turning off there.
    #[serde(default = "default_test_on_check_out")]
    pub test_on_check_out: bool,
}

const fn default_min_idle_pool_size() -> u32 {
    2
}

const fn default_max_lifetime() -> u64 {
    1800
}

const fn default_idle_timeout() -> u64 {
    300
}

const fn default_test_on_check_out() -> bool {
    true
}

impl DbConnectionParams for Database {
    fn get_username(&self) -> &str {
        &self.username
    }
    fn get_password(&self) -> Secret<String> {
        self.password.clone()
    }
    fn get_host(&self) -> &str {
        &self.host
    }
    fn get_port(&self) -> u16 {
        self.port
    }
    fn get_dbname(&self) -> &str {
        &self.dbname
    }
}

#[derive(Debug, serde::Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "PascalCase")]
pub enum QueueStrategy {
    #[default]
    Fifo,
    Lifo,
}

impl From<QueueStrategy> for bb8::QueueStrategy {
    fn from(value: QueueStrategy) -> Self {
        match value {
            QueueStrategy::Fifo => Self::Fifo,
            QueueStrategy::Lifo => Self::Lifo,
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: Secret::<String>::default(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            max_pool_size: 5,
            connection_timeout: 10,
            queue_strategy: QueueStrategy::default(),
            min_idle_pool_size: default_min_idle_pool_size(),
            max_lifetime: default_max_lifetime(),
            idle_timeout: default_idle_timeout(),
            test_on_check_out: default_test_on_check_out(),
        }
    }
}

impl<T: DatabaseStore> MasterKeyInterface for kv_router_store::KVRouterStore<T> {
    fn get_master_key(&self) -> &[u8] {
        self.master_key().peek()
    }
}

impl<T: DatabaseStore> MasterKeyInterface for RouterStore<T> {
    fn get_master_key(&self) -> &[u8] {
        self.master_key().peek()
    }
}

/// Default dummy key for MockDb
impl MasterKeyInterface for MockDb {
    fn get_master_key(&self) -> &[u8] {
        self.master_key()
    }
}
