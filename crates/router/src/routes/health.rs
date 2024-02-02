use actix_web::{web, HttpRequest};
use api_models::health_check::RouterHealthCheckResponse;
use router_env::{instrument, logger, tracing, Flow};

use super::app;
use crate::{
    core::{api_locking, health_check::HealthCheckInterface},
    errors::{self, RouterResponse},
    routes::metrics,
    services::{api, authentication as auth},
};
/// .
// #[logger::instrument(skip_all, name = "name1", level = "warn", fields( key1 = "val1" ))]
#[instrument(skip_all)]
// #[actix_web::get("/health")]
pub async fn health() -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(&metrics::CONTEXT, 1, &[]);
    logger::info!("Health was called");
    actix_web::HttpResponse::Ok().body("health is good")
}

#[instrument(skip_all, fields(flow = ?Flow::DeepHealthCheck))]
/// Performs a deep health check by adding a health metric, creating a flow for deep health check, and executing the deep_health_check_func within a server_wrap.
pub async fn deep_health_check(
    state: web::Data<app::AppState>,
    request: HttpRequest,
) -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(&metrics::CONTEXT, 1, &[]);

    let flow = Flow::DeepHealthCheck;

    Box::pin(api::server_wrap(
        flow,
        state,
        &request,
        (),
        |state, _, _| deep_health_check_func(state),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously performs a deep health check on the application's components including the database, Redis, and locker. If the "olap" feature is enabled, it also performs a health check on the analytics component. It then constructs a RouterHealthCheckResponse containing the status of each component and returns it as an ApplicationResponse in a RouterResponse.
async fn deep_health_check_func(state: app::AppState) -> RouterResponse<RouterHealthCheckResponse> {
    logger::info!("Deep health check was called");

    logger::debug!("Database health check begin");

    let db_status = state.health_check_db().await.map(|_| true).map_err(|err| {
        error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
            component: "Database",
            message: err.to_string()
        })
    })?;

    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = state
        .health_check_redis()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Redis",
                message: err.to_string()
            })
        })?;

    logger::debug!("Redis health check end");

    logger::debug!("Locker health check begin");

    let locker_status = state
        .health_check_locker()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Locker",
                message: err.to_string()
            })
        })?;

    #[cfg(feature = "olap")]
    let analytics_status = state
        .health_check_analytics()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Analytics",
                message: err.to_string()
            })
        })?;

    let outgoing_check = state
        .health_check_outgoing()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Outgoing Request",
                message: err.to_string()
            })
        })?;

    logger::debug!("Locker health check end");

    let response = RouterHealthCheckResponse {
        database: db_status,
        redis: redis_status,
        locker: locker_status,
        #[cfg(feature = "olap")]
        analytics: analytics_status,
        outgoing_request: outgoing_check,
    };

    Ok(api::ApplicationResponse::Json(response))
}
