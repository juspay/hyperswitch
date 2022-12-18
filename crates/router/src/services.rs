pub mod api;
pub mod encryption;
pub mod logger;

use std::sync::Arc;

pub use self::{api::*, encryption::*};
use crate::connection::{diesel_make_pg_pool, PgPool};

#[derive(Clone)]
pub struct Store {
    pub master_pool: PgPool,
    #[cfg(feature = "olap")]
    pub replica_pool: PgPool,
    pub redis_conn: Arc<redis_interface::RedisConnectionPool>,
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
    pub async fn new(config: &crate::configs::settings::Settings, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(&config.master_database, test_transaction).await,
            #[cfg(feature = "olap")]
            replica_pool: diesel_make_pg_pool(&config.replica_database, test_transaction).await,
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
        // Example: {shard_5}_drainer_stream
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }
}
