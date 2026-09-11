//! Two probes. [`health`] is liveness - it touches nothing, because a liveness check that dialled
//! the chat provider would turn every third-party blip into a restart loop. [`deep_health_check`]
//! is readiness and checks the database only: chat and email are destinations this service
//! delivers to and a failing one is already reported per request, but without its own state store
//! there is nothing to serve.
//!
//! That split is what lets the pool be built without connecting - a database briefly away makes an
//! instance unready rather than dead. Mirrors the router's `/health` and `/health/ready`.

use std::time::Duration;

use actix_web::web;
use router_env::{instrument, tracing};

use crate::{logger, state::AppState};

/// How long the probe waits for a connection before answering unready.
///
/// Deliberately shorter than the pool's own connection timeout. `Pool::get` waits that full budget
/// before giving up, so a probe inheriting it answers long after a `timeoutSeconds: 1` readiness
/// check has already given up and recorded nothing — the 503 would never be seen.
const PROBE_BUDGET: Duration = Duration::from_millis(900);

#[instrument(skip_all)]
pub async fn health() -> impl actix_web::Responder {
    logger::info!("Observability health was called");
    actix_web::HttpResponse::Ok().body("Observability health is good")
}

/// Report whether this instance can reach the observability database.
///
/// Answers `503` rather than `500` when it cannot: the service itself is fine, and the condition is
/// expected to clear without intervention. The reason reaches the log and not the body — a probe
/// response is read by anyone who can reach the port, and a connection error names the host, the
/// database and the role.
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
