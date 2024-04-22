pub mod accumulator;
mod core;
pub mod metrics;
pub use accumulator::{AuthEventMetricAccumulator, AuthEventMetricsAccumulator};
pub trait AuthEventAnalytics: metrics::AuthEventMetricAnalytics {}

pub use self::core::get_metrics;
