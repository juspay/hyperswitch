#[doc(inline)]
pub use router_env::*;

pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    /// Setup logging sub-system
    pub fn setup(
        conf: &config::Log,
    ) -> error_stack::Result<TelemetryGuard, router_env::opentelemetry::metrics::MetricsError> {
        Ok(router_env::setup(conf, router_env::service_name!(), [])?)
    }
}
