//! The guarded route tree.
//!
//! Exposed as a [`Scope`] factory rather than as raw handlers, so the standalone binary and a
//! future in-router mount share one definition and cannot drift.

use actix_web::{web, HttpRequest, HttpResponse, Scope};
use serde::Serialize;

use crate::{auth, services, state::AppState};

/// The service's routes, all of them behind the internal API key.
pub struct Alerts;

impl Alerts {
    /// Build the guarded scope.
    ///
    /// Everything mounted here is authenticated. Anything that must be reachable without
    /// credentials belongs in [`crate::health_check`], as its own unguarded scope — not as a path
    /// exception inside a guard, which is where auth bypasses come from.
    pub fn server(state: AppState) -> Scope {
        web::scope("/alerts")
            .app_data(web::Data::new(state))
            .service(web::resource("/ping").route(web::get().to(ping)))
    }
}

/// The response to `GET /alerts/ping`.
#[derive(Debug, Serialize)]
struct PingResponse {
    message: &'static str,
}

/// Proves the guard works end to end, and nothing else.
///
/// Temporary. Delete this when the webhook lands in hyperswitch-cloud#23116 — it exists so the
/// scaffold has something authenticated to exercise, and deliberately does *not* squat on
/// `/notify`, which that ticket owns.
async fn ping(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |_state, ()| async {
            Ok(PingResponse {
                message: "alerts is reachable",
            })
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
