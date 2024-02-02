use api_models::analytics::sdk_events::SdkEventMetricsBucketValue;
use router_env::logger;

use super::metrics::SdkEventMetricRow;

#[derive(Debug, Default)]
pub struct SdkEventMetricsAccumulator {
    pub payment_attempts: CountAccumulator,
    pub payment_success: CountAccumulator,
    pub payment_methods_call_count: CountAccumulator,
    pub average_payment_time: AverageAccumulator,
    pub sdk_initiated_count: CountAccumulator,
    pub sdk_rendered_count: CountAccumulator,
    pub payment_method_selected_count: CountAccumulator,
    pub payment_data_filled_count: CountAccumulator,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AverageAccumulator {
    pub total: u32,
    pub count: u32,
}

pub trait SdkEventMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl SdkEventMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
        /// Adds the metrics count from the given SdkEventMetricRow to the count of the current instance.
    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow) {
        self.count = match (self.count, metrics.count) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        }
    }
    #[inline]
        /// Consumes the result of a count operation and attempts to convert it into a u64,
    /// returning the result as a `MetricOutput`.
    fn collect(self) -> Self::MetricOutput {
        self.count.and_then(|i| u64::try_from(i).ok())
    }
}

impl SdkEventMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

        /// Add the metrics from the given SdkEventMetricRow to the accumulator, updating the total and count.
    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow) {
        let total = metrics
            .total
            .as_ref()
            .and_then(bigdecimal::ToPrimitive::to_u32);
        let count = metrics.count.and_then(|total| u32::try_from(total).ok());

        match (total, count) {
            (Some(total), Some(count)) => {
                self.total += total;
                self.count += count;
            }
            _ => {
                logger::error!(message="Dropping metrics for average accumulator", metric=?metrics);
            }
        }
    }

        /// Calculates the average value of the collected metrics.
    fn collect(self) -> Self::MetricOutput {
        if self.count == 0 {
            None
        } else {
            Some(f64::from(self.total) / f64::from(self.count))
        }
    }
}

impl SdkEventMetricsAccumulator {
    #[allow(dead_code)]
    /// Collects the individual metrics and returns a `SdkEventMetricsBucketValue` containing the aggregated values.
    pub fn collect(self) -> SdkEventMetricsBucketValue {
        SdkEventMetricsBucketValue {
            payment_attempts: self.payment_attempts.collect(),
            payment_success_count: self.payment_success.collect(),
            payment_methods_call_count: self.payment_methods_call_count.collect(),
            average_payment_time: self.average_payment_time.collect(),
            sdk_initiated_count: self.sdk_initiated_count.collect(),
            sdk_rendered_count: self.sdk_rendered_count.collect(),
            payment_method_selected_count: self.payment_method_selected_count.collect(),
            payment_data_filled_count: self.payment_data_filled_count.collect(),
        }
    }
}
