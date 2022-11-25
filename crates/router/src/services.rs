pub mod api;
pub mod encryption;
pub mod logger;
pub mod redis;

use std::sync::Arc;

pub use self::{api::*, encryption::*};

#[derive(Clone)]
pub struct Store {
    pub master_pool: crate::db::SqlDb,
    #[cfg(feature = "olap")]
    pub replica_pool: crate::db::SqlDb,
    pub redis_conn: Arc<crate::services::redis::RedisConnectionPool>,
    #[cfg(feature = "kv_store")]
    pub(crate) config: StoreConfig,
}

#[cfg(feature = "kv_store")]
#[derive(Clone)]
pub(crate) struct StoreConfig {
    pub(crate) drainer_stream_name: String,
    pub(crate) drainer_num_partitions: u8,
}

impl Store {
    pub async fn new(config: &crate::configs::settings::Settings) -> Self {
        Self {
            master_pool: crate::db::SqlDb::new(&config.master_database).await,
            #[cfg(feature = "olap")]
            replica_pool: crate::db::SqlDb::new(&config.replica_database).await,
            redis_conn: Arc::new(crate::connection::redis_connection(config).await),
            #[cfg(feature = "kv_store")]
            config: StoreConfig {
                drainer_stream_name: config.drainer.stream_name.clone(),
                drainer_num_partitions: config.drainer.num_partitions,
            },
        }
    }

    #[cfg(feature = "kv_store")]
    pub fn drainer_stream(&self, shard_key: &str) -> String {
        // "{shard_key}_stream_name"
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }
}
