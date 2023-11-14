pub mod accumulator;
mod core;
pub mod filters;
pub mod metrics;
pub mod types;
pub use accumulator::{PaymentMetricAccumulator, PaymentMetricsAccumulator};

pub trait PaymentAnalytics:
    metrics::PaymentMetricAnalytics + filters::PaymentFilterAnalytics
{
}

pub use self::core::get_metrics;
