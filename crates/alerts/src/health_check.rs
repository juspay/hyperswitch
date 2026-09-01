//! Liveness, on its own unguarded scope.
//!
//! Separate from [`crate::routes`] because probes do not carry credentials. Making that a
//! *structural* property — a scope with no guard on it — rather than a path exception inside the
//! guard means it stays true when someone adds `/healthz`, or a trailing slash, or a new route
//! next to this one. The router does the same: its own `Health` scope does not go through
//! `server_wrap`.
//!
//! Liveness only. A readiness check that dials the chat provider and the mail backend sounds
//! thorough and turns every third-party blip into a restart loop; the point of this service is to
//! be up when its dependencies are flaky.

use actix_web::{web, Scope};
use router_env::{instrument, tracing};

use crate::logger;

/// The unauthenticated health scope.
pub struct Health;

impl Health {
    /// Build the health scope.
    pub fn server() -> Scope {
        web::scope("health").service(web::resource("").route(web::get().to(health)))
    }
}

#[instrument(skip_all)]
async fn health() -> impl actix_web::Responder {
    logger::info!("Alerts health was called");
    actix_web::HttpResponse::Ok().body("Alerts health is good")
}
