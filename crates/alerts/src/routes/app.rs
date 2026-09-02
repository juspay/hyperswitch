//! The whole route tree, in one place.
//!
//! Every route this service serves is visible in this file — mirroring
//! `crates/router/src/routes/app.rs`, where each area is a unit struct with a `server()` returning
//! its [`Scope`]. Handlers live in the sibling modules; only the shape of the tree lives here, so
//! "what does this service expose, and what guards it?" is one file, not a search.
//!
//! Exposed as `Scope` factories rather than as an assembled `App` so that the standalone binary
//! and a future in-router mount share one definition and cannot drift.

use actix_web::{web, Scope};

use crate::{
    errors::types::{ApiError, ApiErrorResponse},
    logger,
    routes::{health_check, notify},
    state::AppState,
};

/// The service's routes, all of them behind the internal API key.
pub struct Alerts;

impl Alerts {
    /// Build the guarded scope.
    ///
    /// Everything mounted here is authenticated. Anything that must be reachable without
    /// credentials belongs in [`Health`] instead — not as a path exception inside a guard, which
    /// is where auth bypasses come from.
    ///
    /// One route per channel, and the path names the channel: `/chat/notify` takes a chat body and
    /// `/email/notify` takes an email one. See [`crate::types`] for why the alternative — one
    /// `/notify/{id}` over a tagged body — was rejected.
    pub fn server(state: AppState) -> Scope {
        web::scope("/alerts")
            .app_data(web::Data::new(state))
            .app_data(json_config())
            .service(
                web::scope("/chat")
                    .service(web::resource("/notify").route(web::post().to(notify::chat))),
            )
            .service(
                web::scope("/email")
                    .service(web::resource("/notify").route(web::post().to(notify::email))),
            )
    }
}

/// Make a malformed body render like every other error this service returns.
///
/// Without this, actix answers its own plain-text 400 and a caller has two error formats to parse
/// depending on how wrong it got things.
///
/// The parse failure itself goes to the log and not to the client, following
/// [`crate::services::server_wrap`]: serde's message quotes the offending part of the body, and a
/// body here carries merchant ids and payment volumes.
fn json_config() -> web::JsonConfig {
    web::JsonConfig::default().error_handler(|error, request| {
        logger::warn!(
            path = %request.path(),
            error = %error,
            "Request rejected: the body could not be parsed"
        );

        ApiErrorResponse::BadRequest(ApiError::new(
            "IR",
            4,
            "The request body could not be parsed",
        ))
        .into()
    })
}

/// Liveness, deliberately unauthenticated.
pub struct Health;

impl Health {
    /// Build the unguarded health scope.
    ///
    /// Separate from [`Alerts`] because probes do not carry credentials. Keeping it a distinct
    /// scope makes "unauthenticated" a structural property visible right here in the route tree,
    /// rather than a condition buried in a guard — so it stays true when someone adds `/healthz`,
    /// a trailing slash, or a route next to this one.
    pub fn server() -> Scope {
        web::scope("health").service(web::resource("").route(web::get().to(health_check::health)))
    }
}
