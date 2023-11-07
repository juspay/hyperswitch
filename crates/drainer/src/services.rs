use std::sync::Arc;

use crate::connection::{diesel_make_pg_pool, PgPool};

#[derive(Clone)]
pub struct Store {
    pub master_pool: PgPool,
    pub redis_conn: Arc<redis_interface::RedisConnectionPool>,
    pub config: StoreConfig,
    pub request_id: Option<String>,
}

#[derive(Clone)]
pub struct StoreConfig {
    pub drainer_stream_name: String,
    pub drainer_num_partitions: u8,
}

impl Store {
    pub async fn new(config: &crate::settings::Settings, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(
                &config.master_database,
                test_transaction,
                #[cfg(feature = "kms")]
                external_services::kms::get_kms_client(&config.kms).await,
            )
            .await,
            redis_conn: Arc::new(crate::connection::redis_connection(config).await),
            config: StoreConfig {
                drainer_stream_name: config.drainer.stream_name.clone(),
                drainer_num_partitions: config.drainer.num_partitions,
            },
            request_id: None,
        }
    }

    pub fn drainer_stream(&self, shard_key: &str) -> String {
        // Example: {shard_5}_drainer_stream
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }
}
