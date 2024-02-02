use api_models::analytics::payments::{ErrorResult, PaymentMetricsBucketValue};
use bigdecimal::ToPrimitive;
use diesel_models::enums as storage_enums;
use router_env::logger;

use super::{distribution::PaymentDistributionRow, metrics::PaymentMetricRow};

#[derive(Debug, Default)]
pub struct PaymentMetricsAccumulator {
    pub payment_success_rate: SuccessRateAccumulator,
    pub payment_count: CountAccumulator,
    pub payment_success: CountAccumulator,
    pub processed_amount: SumAccumulator,
    pub avg_ticket_size: AverageAccumulator,
    pub payment_error_message: ErrorDistributionAccumulator,
    pub retries_count: CountAccumulator,
    pub retries_amount_processed: SumAccumulator,
    pub connector_success_rate: SuccessRateAccumulator,
}

#[derive(Debug, Default)]
pub struct ErrorDistributionRow {
    pub count: i64,
    pub total: i64,
    pub error_message: String,
}

#[derive(Debug, Default)]
pub struct ErrorDistributionAccumulator {
    pub error_vec: Vec<ErrorDistributionRow>,
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

pub trait PaymentDistributionAccumulator {
    type DistributionOutput;

    fn add_distribution_bucket(&mut self, distribution: &PaymentDistributionRow);

    fn collect(self) -> Self::DistributionOutput;
}

impl PaymentDistributionAccumulator for ErrorDistributionAccumulator {
    type DistributionOutput = Option<Vec<ErrorResult>>;

        /// Adds a new payment distribution bucket to the error vector.
    fn add_distribution_bucket(&mut self, distribution: &PaymentDistributionRow) {
        self.error_vec.push(ErrorDistributionRow {
            count: distribution.count.unwrap_or_default(),
            total: distribution
                .total
                .clone()
                .map(|i| i.to_i64().unwrap_or_default())
                .unwrap_or_default(),
            error_message: distribution.error_message.clone().unwrap_or("".to_string()),
        })
    }

    /// Method to collect error results and calculate percentage
    fn collect(mut self) -> Self::DistributionOutput {
        if self.error_vec.is_empty() {
            None
        } else {
            self.error_vec.sort_by(|a, b| b.count.cmp(&a.count));
            let mut res: Vec<ErrorResult> = Vec::new();
            for val in self.error_vec.into_iter() {
                let perc = f64::from(u32::try_from(val.count).ok()?) * 100.0
                    / f64::from(u32::try_from(val.total).ok()?);

                res.push(ErrorResult {
                    reason: val.error_message,
                    count: val.count,
                    percentage: (perc * 100.0).round() / 100.0,
                })
            }

            Some(res)
        }
    }
}

impl PaymentMetricAccumulator for SuccessRateAccumulator {
    type MetricOutput = Option<f64>;

        /// Adds the metrics from a PaymentMetricRow to the existing bucket metrics.
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::AttemptStatus::Charged {
                self.success += metrics.count.unwrap_or_default();
            }
        };
        self.total += metrics.count.unwrap_or_default();
    }

        /// Calculates the success rate as a percentage based on the number of successful attempts and the total number of attempts.
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
        /// Adds the count from the given PaymentMetricRow to the count of the current instance
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        self.count = match (self.count, metrics.count) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        }
    }
    #[inline]
        /// This method collects the result of the count and then attempts to convert it into a u64, returning an Option<u64>.
    fn collect(self) -> Self::MetricOutput {
        self.count.and_then(|i| u64::try_from(i).ok())
    }
}

impl PaymentMetricAccumulator for SumAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
        /// Adds the metric values from the given `PaymentMetricRow` to the existing total metric values in the struct.
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
        /// Converts the `total` field into a `u64` and returns it as the `MetricOutput` type.
    fn collect(self) -> Self::MetricOutput {
        u64::try_from(self.total.unwrap_or(0)).ok()
    }
}

impl PaymentMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

        /// Adds the total and count metrics from the provided PaymentMetricRow to the total and count
    /// metrics of the current instance.
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

        /// Calculates and returns the average of the collected values. If no values have been collected, returns None.
    fn collect(self) -> Self::MetricOutput {
        if self.count == 0 {
            None
        } else {
            Some(f64::from(self.total) / f64::from(self.count))
        }
    }
}

impl PaymentMetricsAccumulator {
        /// Collects the payment metrics bucket values and returns a new `PaymentMetricsBucketValue` containing the collected values
    pub fn collect(self) -> PaymentMetricsBucketValue {
        PaymentMetricsBucketValue {
            payment_success_rate: self.payment_success_rate.collect(),
            payment_count: self.payment_count.collect(),
            payment_success_count: self.payment_success.collect(),
            payment_processed_amount: self.processed_amount.collect(),
            avg_ticket_size: self.avg_ticket_size.collect(),
            payment_error_message: self.payment_error_message.collect(),
            retries_count: self.retries_count.collect(),
            retries_amount_processed: self.retries_amount_processed.collect(),
            connector_success_rate: self.connector_success_rate.collect(),
        }
    }
}
