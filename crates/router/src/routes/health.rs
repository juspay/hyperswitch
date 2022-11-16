use router_env::{
    logger,
    tracing::{self, instrument},
};

use crate::routes::metrics::HEALTH_METRIC;

/// .
// #[logger::instrument(skip_all, name = "name1", level = "warn", fields( key1 = "val1" ))]
#[instrument(skip_all)]
// #[actix_web::get("/health")]
pub async fn health() -> impl actix_web::Responder {
    HEALTH_METRIC.add(1, &[]);
    logger::info!("Health was called");
    actix_web::HttpResponse::Ok().body("health is good")
}
