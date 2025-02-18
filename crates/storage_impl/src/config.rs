use common_utils::{id_type, DbConnectionParams};
use masking::Secret;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub queue_strategy: QueueStrategy,
    pub min_idle: Option<u32>,
    pub max_lifetime: Option<u64>,
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

pub trait TenantConfig: Send + Sync {
    fn get_tenant_id(&self) -> &id_type::TenantId;
    fn get_schema(&self) -> &str;
    fn get_accounts_schema(&self) -> &str;
    fn get_redis_key_prefix(&self) -> &str;
    fn get_clickhouse_database(&self) -> &str;
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
            pool_size: 5,
            connection_timeout: 10,
            queue_strategy: QueueStrategy::default(),
            min_idle: None,
            max_lifetime: None,
        }
    }
}
