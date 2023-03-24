use crate::routes::metrics;
#[cfg(feature = "profiling")]
use crate::services::profiler;
use router_env::{instrument, logger, tracing};
/// .
// #[logger::instrument(skip_all, name = "name1", level = "warn", fields( key1 = "val1" ))]
#[instrument(skip_all)]
// #[actix_web::get("/health")]
pub async fn health() -> impl actix_web::Responder {
    metrics::HEALTH_METRIC.add(&metrics::CONTEXT, 1, &[]);
    logger::info!("Health was called");
    actix_web::HttpResponse::Ok().body("health is good")
}

#[cfg(feature = "profiling")]
#[instrument(skip_all)]
pub async fn profiler_out() -> impl actix_web::Responder {
    let mut flamegraph = Vec::<u8>::new();
    if let Ok(report) = profiler::PROFILER_GUARD
        .get()
        .expect("Profiler failure")
        .report()
        .build()
    {
        report.flamegraph(&mut flamegraph).unwrap();
    }

    actix_web::HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(flamegraph)
}
