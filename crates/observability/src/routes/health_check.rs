use std::time::Duration;

use actix_web::web;
use router_env::{instrument, tracing};

use crate::{logger, state::AppState};

const PROBE_BUDGET: Duration = Duration::from_millis(900);

#[instrument(skip_all)]
pub async fn health() -> impl actix_web::Responder {
    logger::info!("Observability health was called");
    actix_web::HttpResponse::Ok().body("Observability health is good")
}

#[instrument(skip_all)]
pub async fn deep_health_check(state: web::Data<AppState>) -> impl actix_web::Responder {
    match tokio::time::timeout(PROBE_BUDGET, state.database.get()).await {
        Err(_elapsed) => {
            logger::error!("Observability deep health check timed out taking a connection");
            return actix_web::HttpResponse::ServiceUnavailable()
                .body("Observability database is unreachable");
        }
        Ok(result) => match result {
            Ok(_connection) => {
                logger::info!("Observability deep health check passed");
                actix_web::HttpResponse::Ok().body("Observability deep health is good")
            }
            Err(error) => {
                logger::error!(
                    ?error,
                    "Observability deep health check failed: database unreachable"
                );
                actix_web::HttpResponse::ServiceUnavailable()
                    .body("Observability database is unreachable")
            }
        },
    }
}
