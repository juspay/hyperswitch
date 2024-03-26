pub mod accumulator;
mod core;
pub mod events;
pub mod filters;
pub mod metrics;
pub mod types;
pub use accumulator::{SdkEventMetricAccumulator, SdkEventMetricsAccumulator};
pub trait SDKEventAnalytics: events::SdkEventsFilterAnalytics {}
pub trait SdkEventAnalytics:
    metrics::SdkEventMetricAnalytics + filters::SdkEventFilterAnalytics
{
}

pub use self::core::{get_filters, get_metrics, sdk_events_core};
