pub mod accumulator;
mod core;
pub mod filters;
pub mod metrics;
pub mod sankey;
pub mod types;
pub use accumulator::{PaymentIntentMetricAccumulator, PaymentIntentMetricsAccumulator};

pub trait PaymentIntentAnalytics:
    metrics::PaymentIntentMetricAnalytics + filters::PaymentIntentFilterAnalytics
{
}

pub use self::core::{get_filters, get_metrics, get_sankey};
