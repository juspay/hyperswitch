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
    routes::{alerts, health_check},
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
    pub fn server(state: AppState) -> Scope {
        web::scope("/alerts")
            .app_data(web::Data::new(state))
            .service(web::resource("/ping").route(web::get().to(alerts::ping)))
    }
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
