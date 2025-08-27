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
#[instrument(skip_all, fields(flow = ?Flow::HealthCheck))]
// #[actix_web::get("/health")]
pub async fn health() -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(1, &[]);
    logger::info!("Health was called");

    actix_web::HttpResponse::Ok().body("health is good")
}

#[instrument(skip_all, fields(flow = ?Flow::DeepHealthCheck))]
pub async fn deep_health_check(
    state: web::Data<app::AppState>,
    request: HttpRequest,
) -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(1, &[]);

    let flow = Flow::DeepHealthCheck;

    Box::pin(api::server_wrap(
        flow,
        state,
        &request,
        (),
        |state, _: (), _, _| deep_health_check_func(state),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

async fn deep_health_check_func(
    state: app::SessionState,
) -> RouterResponse<RouterHealthCheckResponse> {
    logger::info!("Deep health check was called");

    logger::debug!("Database health check begin");

    let db_status = state.health_check_db().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Database",
            message,
        })
    })?;

    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = state.health_check_redis().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Redis",
            message,
        })
    })?;

    logger::debug!("Redis health check end");

    logger::debug!("Locker health check begin");

    let locker_status = state.health_check_locker().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Locker",
            message,
        })
    })?;

    logger::debug!("Locker health check end");

    logger::debug!("Analytics health check begin");

    #[cfg(feature = "olap")]
    let analytics_status = state.health_check_analytics().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Analytics",
            message,
        })
    })?;

    logger::debug!("Analytics health check end");

    logger::debug!("gRPC health check begin");

    #[cfg(feature = "dynamic_routing")]
    let grpc_health_check = state.health_check_grpc().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "gRPC services",
            message,
        })
    })?;

    logger::debug!("gRPC health check end");

    logger::debug!("Decision Engine health check begin");

    #[cfg(feature = "dynamic_routing")]
    let decision_engine_health_check =
        state
            .health_check_decision_engine()
            .await
            .map_err(|error| {
                let message = error.to_string();
                error.change_context(errors::ApiErrorResponse::HealthCheckError {
                    component: "Decision Engine service",
                    message,
                })
            })?;

    logger::debug!("Decision Engine health check end");

    logger::debug!("Opensearch health check begin");

    #[cfg(feature = "olap")]
    let opensearch_status = state.health_check_opensearch().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Opensearch",
            message,
        })
    })?;

    logger::debug!("Opensearch health check end");

    logger::debug!("Outgoing Request health check begin");

    let outgoing_check = state.health_check_outgoing().await.map_err(|error| {
        let message = error.to_string();
        error.change_context(errors::ApiErrorResponse::HealthCheckError {
            component: "Outgoing Request",
            message,
        })
    })?;

    logger::debug!("Outgoing Request health check end");

    logger::debug!("Unified Connector Service health check begin");

    let unified_connector_service_status = state
        .health_check_unified_connector_service()
        .await
        .map_err(|error| {
            let message = error.to_string();
            error.change_context(errors::ApiErrorResponse::HealthCheckError {
                component: "Unified Connector Service",
                message,
            })
        })?;

    logger::debug!("Unified Connector Service health check end");

    let response = RouterHealthCheckResponse {
        database: db_status.into(),
        redis: redis_status.into(),
        vault: locker_status.into(),
        #[cfg(feature = "olap")]
        analytics: analytics_status.into(),
        #[cfg(feature = "olap")]
        opensearch: opensearch_status.into(),
        outgoing_request: outgoing_check.into(),
        #[cfg(feature = "dynamic_routing")]
        grpc_health_check,
        #[cfg(feature = "dynamic_routing")]
        decision_engine: decision_engine_health_check.into(),
        unified_connector_service: unified_connector_service_status.into(),
    };

    Ok(api::ApplicationResponse::Json(response))
}
