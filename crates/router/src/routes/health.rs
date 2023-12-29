use actix_web::web;
use api_models::health_check::{HealthCheckResponse, KeyCustodianStatus, LockerHealthResponse};
use router_env::{instrument, logger, tracing};

use super::app;
use crate::{routes::metrics, services};
/// .
// #[logger::instrument(skip_all, name = "name1", level = "warn", fields( key1 = "val1" ))]
#[instrument(skip_all)]
// #[actix_web::get("/health")]
pub async fn health() -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(&metrics::CONTEXT, 1, &[]);
    logger::info!("Health was called");
    actix_web::HttpResponse::Ok().body("health is good")
}

#[instrument(skip_all)]
pub async fn deep_health_check(state: web::Data<app::AppState>) -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(&metrics::CONTEXT, 1, &[]);
    let db = &*state.store;
    let mut status_code = 200;
    logger::info!("Deep health check was called");

    logger::debug!("Database health check begin");

    let db_status = match db.health_check_db(db).await {
        Ok(_) => "Health is good".to_string(),
        Err(err) => {
            status_code = 500;
            err.to_string()
        }
    };
    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = match db.health_check_redis(db).await {
        Ok(_) => "Health is good".to_string(),
        Err(err) => {
            status_code = 500;
            err.to_string()
        }
    };

    logger::debug!("Redis health check end");

    logger::debug!("Locker health check begin");

    let (locker_status, key_custodian_status) = match db.health_check_locker(&state).await {
        Ok(status_code) => {
            let status_message = "Health is good".to_string();
            let key_custodian_status = if status_code == 403 {
                KeyCustodianStatus::Locked
            } else {
                KeyCustodianStatus::Unlocked
            };
            (status_message, key_custodian_status)
        }
        Err(err) => {
            status_code = 500;
            (err.to_string(), KeyCustodianStatus::Unavailable)
        }
    };

    logger::debug!("Locker health check end");

    let response = serde_json::to_string(&HealthCheckResponse {
        database: db_status,
        redis: redis_status,
        locker: LockerHealthResponse {
            status: locker_status,
            key_custodian_status,
        },
    })
    .unwrap_or_default();

    if status_code == 200 {
        services::http_response_json(response)
    } else {
        services::http_server_error_json_response(response)
    }
}
