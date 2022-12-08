#[doc(inline)]
pub use router_env::*;
pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    ///
    /// Setup logging sub-system.
    ///
    // TODO (prom-monitoring): Ideally tracing/opentelementry structs shouldn't be pushed out
    // Find an abstraction so that source crate is unaware about underlying implementation
    // https://juspay.atlassian.net/browse/ORCA-345
    pub fn setup(
        conf: &config::Log,
    ) -> Result<TelemetryGuard, router_env::opentelemetry::metrics::MetricsError> {
        router_env::setup(conf, "router", vec!["router", "actix_server"])
    }
}
