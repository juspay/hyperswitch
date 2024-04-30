use std::sync::Arc;

use actix_web::{web, Scope};
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use common_utils::errors::CustomResult;
use diesel_models::{Config, ConfigNew};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use crate::{
    connection::{pg_connection, redis_connection},
    errors::HealthCheckError,
    services::{self, Store},
    Settings,
};

pub const TEST_STREAM_NAME: &str = "TEST_STREAM_0";
pub const TEST_STREAM_DATA: &[(&str, &str)] = &[("data", "sample_data")];

pub struct Health;

impl Health {
    pub fn server(conf: Settings, store: Arc<Store>) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(conf))
            .app_data(web::Data::new(store))
            .service(web::resource("").route(web::get().to(health)))
            .service(web::resource("/ready").route(web::get().to(deep_health_check)))
    }
}

//#\[instrument\(skip_all)]
pub async fn health() -> impl actix_web::Responder {
    logger::info!("Drainer health was called");
    actix_web::HttpResponse::Ok().body("Drainer health is good")
}

//#\[instrument\(skip_all)]
pub async fn deep_health_check(
    conf: web::Data<Settings>,
    store: web::Data<Arc<Store>>,
) -> impl actix_web::Responder {
    match deep_health_check_func(conf, store).await {
        Ok(response) => services::http_response_json(
            serde_json::to_string(&response)
                .map_err(|err| {
                    logger::error!(serialization_error=?err);
                })
                .unwrap_or_default(),
        ),

        Err(err) => services::log_and_return_error_response(err),
    }
}

//#\[instrument\(skip_all)]
pub async fn deep_health_check_func(
    conf: web::Data<Settings>,
    store: web::Data<Arc<Store>>,
) -> Result<DrainerHealthCheckResponse, error_stack::Report<HealthCheckError>> {
    logger::info!("Deep health check was called");

    logger::debug!("Database health check begin");

    let db_status = store.health_check_db().await.map(|_| true).map_err(|err| {
        error_stack::report!(HealthCheckError::DbError {
            message: err.to_string()
        })
    })?;

    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = store
        .health_check_redis(&conf.into_inner())
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(HealthCheckError::RedisError {
                message: err.to_string()
            })
        })?;

    logger::debug!("Redis health check end");

    Ok(DrainerHealthCheckResponse {
        database: db_status,
        redis: redis_status,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DrainerHealthCheckResponse {
    pub database: bool,
    pub redis: bool,
}

#[async_trait::async_trait]
pub trait HealthCheckInterface {
    async fn health_check_db(&self) -> CustomResult<(), HealthCheckDBError>;
    async fn health_check_redis(&self, conf: &Settings) -> CustomResult<(), HealthCheckRedisError>;
}

#[async_trait::async_trait]
impl HealthCheckInterface for Store {
    async fn health_check_db(&self) -> CustomResult<(), HealthCheckDBError> {
        let conn = pg_connection(&self.master_pool).await;

        conn
            .transaction_async(|conn| {
                Box::pin(async move {
                    let query =
                        diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
                    let _x: i32 = query.get_result_async(&conn).await.map_err(|err| {
                        logger::error!(read_err=?err,"Error while reading element in the database");
                        HealthCheckDBError::DbReadError
                    })?;

                    logger::debug!("Database read was successful");

                    let config = ConfigNew {
                        key: "test_key".to_string(),
                        config: "test_value".to_string(),
                    };

                    config.insert(&conn).await.map_err(|err| {
                        logger::error!(write_err=?err,"Error while writing to database");
                        HealthCheckDBError::DbWriteError
                    })?;

                    logger::debug!("Database write was successful");

                    Config::delete_by_key(&conn, "test_key").await.map_err(|err| {
                        logger::error!(delete_err=?err,"Error while deleting element in the database");
                        HealthCheckDBError::DbDeleteError
                    })?;

                    logger::debug!("Database delete was successful");

                    Ok::<_, HealthCheckDBError>(())
                })
            })
            .await?;

        Ok(())
    }

    async fn health_check_redis(&self, conf: &Settings) -> CustomResult<(), HealthCheckRedisError> {
        let redis_conn = redis_connection(conf).await;

        redis_conn
            .serialize_and_set_key_with_expiry("test_key", "test_value", 30)
            .await
            .change_context(HealthCheckRedisError::SetFailed)?;

        logger::debug!("Redis set_key was successful");

        redis_conn
            .get_key("test_key")
            .await
            .change_context(HealthCheckRedisError::GetFailed)?;

        logger::debug!("Redis get_key was successful");

        redis_conn
            .delete_key("test_key")
            .await
            .change_context(HealthCheckRedisError::DeleteFailed)?;

        logger::debug!("Redis delete_key was successful");

        redis_conn
            .stream_append_entry(
                TEST_STREAM_NAME,
                &redis_interface::RedisEntryId::AutoGeneratedID,
                TEST_STREAM_DATA.to_vec(),
            )
            .await
            .change_context(HealthCheckRedisError::StreamAppendFailed)?;

        logger::debug!("Stream append succeeded");

        let output = self
            .redis_conn
            .stream_read_entries(TEST_STREAM_NAME, "0-0", Some(10))
            .await
            .change_context(HealthCheckRedisError::StreamReadFailed)?;
        logger::debug!("Stream read succeeded");

        let (_, id_to_trim) = output
            .get(TEST_STREAM_NAME)
            .and_then(|entries| {
                entries
                    .last()
                    .map(|last_entry| (entries, last_entry.0.clone()))
            })
            .ok_or(error_stack::report!(
                HealthCheckRedisError::StreamReadFailed
            ))?;
        logger::debug!("Stream parse succeeded");

        redis_conn
            .stream_trim_entries(
                TEST_STREAM_NAME,
                (
                    redis_interface::StreamCapKind::MinID,
                    redis_interface::StreamCapTrim::Exact,
                    id_to_trim,
                ),
            )
            .await
            .change_context(HealthCheckRedisError::StreamTrimFailed)?;
        logger::debug!("Stream trim succeeded");

        Ok(())
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckDBError {
    #[error("Error while connecting to database")]
    DbError,
    #[error("Error while writing to database")]
    DbWriteError,
    #[error("Error while reading element in the database")]
    DbReadError,
    #[error("Error while deleting element in the database")]
    DbDeleteError,
    #[error("Unpredictable error occurred")]
    UnknownError,
    #[error("Error in database transaction")]
    TransactionError,
}

impl From<diesel::result::Error> for HealthCheckDBError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::DatabaseError(_, _) => Self::DbError,

            diesel::result::Error::RollbackErrorOnCommit { .. }
            | diesel::result::Error::RollbackTransaction
            | diesel::result::Error::AlreadyInTransaction
            | diesel::result::Error::NotInTransaction
            | diesel::result::Error::BrokenTransactionManager => Self::TransactionError,

            _ => Self::UnknownError,
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckRedisError {
    #[error("Failed to set key value in Redis")]
    SetFailed,
    #[error("Failed to get key value in Redis")]
    GetFailed,
    #[error("Failed to delete key value in Redis")]
    DeleteFailed,
    #[error("Failed to append data to the stream in Redis")]
    StreamAppendFailed,
    #[error("Failed to read data from the stream in Redis")]
    StreamReadFailed,
    #[error("Failed to trim data from the stream in Redis")]
    StreamTrimFailed,
}
