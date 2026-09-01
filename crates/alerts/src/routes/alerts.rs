//! Handlers for the guarded scope. The route tree that mounts them is in [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;

use crate::{auth, services, state::AppState};

/// The response to `GET /alerts/ping`.
#[derive(Debug, Serialize)]
pub struct PingResponse {
    message: &'static str,
}

/// Proves the guard works end to end, and nothing else.
///
/// Temporary. Delete this when the webhook lands in hyperswitch-cloud#23116 — it exists so the
/// scaffold has something authenticated to exercise, and deliberately does *not* squat on
/// `/notify`, which that ticket owns.
pub async fn ping(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
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
