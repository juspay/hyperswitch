use api_models::analytics::refunds::RefundMetricsBucketValue;
use diesel_models::enums as storage_enums;

use super::metrics::RefundMetricRow;
#[derive(Debug, Default)]
pub struct RefundMetricsAccumulator {
    pub refund_success_rate: SuccessRateAccumulator,
    pub refund_count: CountAccumulator,
    pub refund_success: CountAccumulator,
    pub processed_amount: RefundProcessedAmountAccumulator,
}

#[derive(Debug, Default)]
pub struct SuccessRateAccumulator {
    pub success: u32,
    pub total: u32,
}
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}
#[derive(Debug, Default)]
pub struct RefundProcessedAmountAccumulator {
    pub count: Option<i64>,
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

impl RefundMetricAccumulator for RefundProcessedAmountAccumulator {
    type MetricOutput = (Option<u64>, Option<u64>, Option<u64>);
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
        };

        self.count = match (self.count, metrics.count) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        };
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        let total = u64::try_from(self.total.unwrap_or(0)).ok();
        let count = self.count.and_then(|i| u64::try_from(i).ok());

        (total, count, Some(0))
    }
}

impl RefundMetricAccumulator for SuccessRateAccumulator {
    type MetricOutput = (Option<u32>, Option<u32>, Option<f64>);

    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow) {
        if let Some(ref refund_status) = metrics.refund_status {
            if refund_status.as_ref() == &storage_enums::RefundStatus::Success {
                if let Some(success) = metrics
                    .count
                    .and_then(|success| u32::try_from(success).ok())
                {
                    self.success += success;
                }
            }
        };
        if let Some(total) = metrics.count.and_then(|total| u32::try_from(total).ok()) {
            self.total += total;
        }
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total == 0 {
            (None, None, None)
        } else {
            let success = Some(self.success);
            let total = Some(self.total);
            let success_rate = match (success, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };
            (success, total, success_rate)
        }
    }
}

impl RefundMetricsAccumulator {
    pub fn collect(self) -> RefundMetricsBucketValue {
        let (successful_refunds, total_refunds, refund_success_rate) =
            self.refund_success_rate.collect();
        let (refund_processed_amount, refund_processed_count, refund_processed_amount_in_usd) =
            self.processed_amount.collect();
        RefundMetricsBucketValue {
            successful_refunds,
            total_refunds,
            refund_success_rate,
            refund_count: self.refund_count.collect(),
            refund_success_count: self.refund_success.collect(),
            refund_processed_amount,
            refund_processed_amount_in_usd,
            refund_processed_count,
        }
    }
}
