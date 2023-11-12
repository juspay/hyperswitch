use api_models::analytics::payments::PaymentMetricsBucketValue;
use common_enums::enums as storage_enums;
use router_env::logger;

use super::metrics::PaymentMetricRow;

#[derive(Debug, Default)]
pub struct PaymentMetricsAccumulator {
    pub payment_success_rate: SuccessRateAccumulator,
    pub payment_count: CountAccumulator,
    pub payment_success: CountAccumulator,
    pub processed_amount: SumAccumulator,
    pub avg_ticket_size: AverageAccumulator,
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

#[derive(Debug, Default)]
pub struct AverageAccumulator {
    pub total: u32,
    pub count: u32,
}

pub trait PaymentMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl PaymentMetricAccumulator for SuccessRateAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::AttemptStatus::Charged {
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

impl PaymentMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
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

impl PaymentMetricAccumulator for SumAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
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
        u64::try_from(self.total.unwrap_or(0)).ok()
    }
}

impl PaymentMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
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

    fn collect(self) -> Self::MetricOutput {
        if self.count == 0 {
            None
        } else {
            Some(f64::from(self.total) / f64::from(self.count))
        }
    }
}

impl PaymentMetricsAccumulator {
    pub fn collect(self) -> PaymentMetricsBucketValue {
        PaymentMetricsBucketValue {
            payment_success_rate: self.payment_success_rate.collect(),
            payment_count: self.payment_count.collect(),
            payment_success_count: self.payment_success.collect(),
            payment_processed_amount: self.processed_amount.collect(),
            avg_ticket_size: self.avg_ticket_size.collect(),
        }
    }
}
