use api_models::analytics::active_payments::ActivePaymentsMetricsBucketValue;

use super::metrics::ActivePaymentsMetricRow;

#[derive(Debug, Default)]
pub struct ActivePaymentsMetricsAccumulator {
    pub active_payments: CountAccumulator,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}

pub trait ActivePaymentsMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &ActivePaymentsMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl ActivePaymentsMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &ActivePaymentsMetricRow) {
        self.count = match (self.count, metrics.count) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        }
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        self.count.and_then(|i| u64::try_from(i).ok())
    }
}

impl ActivePaymentsMetricsAccumulator {
    #[allow(dead_code)]
    pub fn collect(self) -> ActivePaymentsMetricsBucketValue {
        ActivePaymentsMetricsBucketValue {
            active_payments: self.active_payments.collect(),
        }
    }
}
