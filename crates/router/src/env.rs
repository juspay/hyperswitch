#[doc(inline)]
pub use router_env::*;
pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    ///
    /// Setup logging sub-system.
    ///
    // TODO (prom-monitoring): Ideally tracing/opentelementry structs shouldn't be pushed out.
    // Return a custom error type instead of `opentelemetry::metrics::MetricsError`.
    pub fn setup(
        conf: &config::Log,
    ) -> Result<TelemetryGuard, router_env::opentelemetry::metrics::MetricsError> {
        router_env::setup(
            conf,
            "router",
            vec![
                "router",
                "actix_server",
                "api_models",
                "common_utils",
                "masking",
                "redis_interface",
                "router_derive",
                "router_env",
                "storage_models",
            ],
        )
    }
}
