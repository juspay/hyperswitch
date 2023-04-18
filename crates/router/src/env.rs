#[doc(inline)]
pub use router_env::*;
pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    // TODO (prom-monitoring): Ideally tracing/opentelemetry structs shouldn't be pushed out.
    // Return a custom error type instead of `opentelemetry::metrics::MetricsError`.
    /// Setup logging sub-system.
    pub fn setup(
        conf: &config::Log,
    ) -> Result<TelemetryGuard, router_env::opentelemetry::metrics::MetricsError> {
        router_env::setup(conf, router_env::service_name!(), ["actix_server"])
    }
}
