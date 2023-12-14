pub mod accumulator;
mod core;
pub mod distribution;
pub mod filters;
pub mod metrics;
pub mod types;
pub use accumulator::{
    PaymentDistributionAccumulator, PaymentMetricAccumulator, PaymentMetricsAccumulator,
};

pub trait PaymentAnalytics:
    metrics::PaymentMetricAnalytics + filters::PaymentFilterAnalytics
{
}

pub use self::core::{get_filters, get_metrics};
