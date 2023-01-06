#[doc(inline)]
pub use router_env::*;
pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    ///
    /// Setup logging sub-system.
    ///
    pub fn setup(
        conf: &config::Log,
    ) -> Result<TelemetryGuard, router_env::opentelemetry::metrics::MetricsError> {
        router_env::setup(conf, "router", vec!["router", "actix_server"])
    }
}
