use api_models::analytics::refunds::RefundMetricsBucketValue;
use diesel_models::enums as storage_enums;

use super::metrics::RefundMetricRow;
#[derive(Debug, Default)]
pub struct RefundMetricsAccumulator {
    pub refund_success_rate: SuccessRateAccumulator,
    pub refund_count: CountAccumulator,
    pub refund_success: CountAccumulator,
    pub processed_amount: SumAccumulator,
}

#[derive(Debug, Default)]
pub struct SuccessRateAccumulator {
    pub success: i64,
    pub total: i64,
}
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct SumAccumulator {
    pub total: Option<i64>,
}

pub trait RefundMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl RefundMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow) {
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

impl RefundMetricAccumulator for SumAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow) {
        self.total = match (
            self.total,
            metrics
                .total
                .as_ref()
                .and_then(bigdecimal::ToPrimitive::to_i64),
        ) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        }
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        self.total.and_then(|i| u64::try_from(i).ok())
    }
}

impl RefundMetricAccumulator for SuccessRateAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow) {
        if let Some(ref refund_status) = metrics.refund_status {
            if refund_status.as_ref() == &storage_enums::RefundStatus::Success {
                self.success += metrics.count.unwrap_or_default();
            }
        };
        self.total += metrics.count.unwrap_or_default();
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total <= 0 {
            None
        } else {
            Some(
                f64::from(u32::try_from(self.success).ok()?) * 100.0
                    / f64::from(u32::try_from(self.total).ok()?),
            )
        }
    }
}

impl RefundMetricsAccumulator {
    pub fn collect(self) -> RefundMetricsBucketValue {
        RefundMetricsBucketValue {
            refund_success_rate: self.refund_success_rate.collect(),
            refund_count: self.refund_count.collect(),
            refund_success_count: self.refund_success.collect(),
            refund_processed_amount: self.processed_amount.collect(),
        }
    }
}
