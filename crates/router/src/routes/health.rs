use super::app;
use actix_web::web;
use router_env::{instrument, logger, tracing};

use crate::routes::metrics;

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
    logger::info!("Deep health check was called");

    logger::debug!("Database health check begin");

    let db_status = match db.health_check_db(db).await {
        Ok(_) => "Health is good".to_string(),
        Err(err) => err.to_string(),
    };
    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = match db.health_check_redis(db).await {
        Ok(_) => "Health is good".to_string(),
        Err(err) => err.to_string(),
    };

    logger::debug!("Redis health check end");

    logger::debug!("Locker health check begin");

    let locker_status = match db.health_check_locker(&state).await {
        Ok(status_code) => {
            let mut status_message = "Health is good".to_string();
            if status_code == 403 {
                status_message = format!("{}; Key custodian locked", status_message);
            }
            status_message
        }
        Err(err) => err.to_string(),
    };

    logger::debug!("Locker health check end");

    actix_web::HttpResponse::Ok().body(format!(
        "Database - {}\nRedis - {}\nLocker - {}",
        db_status, redis_status, locker_status
    ))
}
