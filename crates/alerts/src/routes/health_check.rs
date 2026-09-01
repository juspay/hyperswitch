//! Health handlers. The route tree that mounts them is in [`crate::routes::app`].
//!
//! Liveness only. A readiness check that dials the chat provider and the mail backend sounds
//! thorough and turns every third-party blip into a restart loop; the point of this service is to
//! be up when its dependencies are flaky.

use router_env::{instrument, tracing};

use crate::logger;

#[instrument(skip_all)]
pub async fn health() -> impl actix_web::Responder {
    logger::info!("Alerts health was called");
    actix_web::HttpResponse::Ok().body("Alerts health is good")
}
