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
    pub drainer_num_partitions: u8,
}

impl Store {
    /// # Panics
    ///
    /// Panics if there is a failure while obtaining the HashiCorp client using the provided configuration.
    /// This panic indicates a critical failure in setting up external services, and the application cannot proceed without a valid HashiCorp client.
    ///
    pub async fn new(config: &crate::settings::Settings, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(
                &config.master_database,
                test_transaction,
                #[cfg(feature = "aws_kms")]
                external_services::aws_kms::core::get_aws_kms_client(&config.kms).await,
                #[cfg(feature = "hashicorp-vault")]
                #[allow(clippy::expect_used)]
                external_services::hashicorp_vault::core::get_hashicorp_client(&config.hc_vault)
                    .await
                    .expect("Failed while getting hashicorp client"),
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
