use std::sync::Arc;

use crate::{
    connection::{diesel_make_pg_pool, PgPool},
    settings::AppState,
};

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
    pub async fn new(state: &AppState, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(
                &state.conf.master_database.into_inner(),
                test_transaction,
            )
            .await,
            redis_conn: Arc::new(crate::connection::redis_connection(&state.conf).await),
            config: StoreConfig {
                drainer_stream_name: state.conf.drainer.stream_name.clone(),
                drainer_num_partitions: state.conf.drainer.num_partitions,
            },
            request_id: None,
        }
    }
}
