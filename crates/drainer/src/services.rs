use std::sync::Arc;

use actix_web::{body, HttpResponse, ResponseError};
use error_stack::Report;

use crate::{
    connection::{diesel_make_pg_pool, PgPool},
    logger,
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
    pub drainer_error_stream_name: String,
    pub drainer_num_partitions: u8,
    pub drainer_error_num_partitions: u8,
}

impl Store {
    /// # Panics
    ///
    /// Panics if there is a failure while obtaining the HashiCorp client using the provided configuration.
    /// This panic indicates a critical failure in setting up external services, and the application cannot proceed without a valid HashiCorp client.
    ///
    pub async fn new(config: &crate::Settings, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(config.master_database.get_inner(), test_transaction)
                .await,
            redis_conn: Arc::new(crate::connection::redis_connection(config).await),
            config: StoreConfig {
                drainer_stream_name: config.drainer.stream_name.clone(),
                drainer_num_partitions: config.drainer.num_partitions,
                drainer_error_num_partitions: config.drainer.num_err_partitions,
                drainer_error_stream_name: config.drainer.error_stream_name.clone(),
            },
            request_id: None,
        }
    }
}

pub fn log_and_return_error_response<T>(error: Report<T>) -> HttpResponse
where
    T: error_stack::Context + ResponseError + Clone,
{
    logger::error!(?error);
    let body = serde_json::json!({
        "message": error.to_string()
    })
    .to_string();
    HttpResponse::InternalServerError()
        .content_type(mime::APPLICATION_JSON)
        .body(body)
}

pub fn http_response_json<T: body::MessageBody + 'static>(response: T) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(mime::APPLICATION_JSON)
        .body(response)
}
