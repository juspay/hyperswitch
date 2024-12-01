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
    pub processed_amount: ProcessedAmountAccumulator,
    pub avg_ticket_size: AverageAccumulator,
    pub payment_error_message: ErrorDistributionAccumulator,
    pub retries_count: CountAccumulator,
    pub retries_amount_processed: RetriesAmountAccumulator,
    pub connector_success_rate: SuccessRateAccumulator,
    pub payments_distribution: PaymentsDistributionAccumulator,
    pub failure_reasons_distribution: FailureReasonsDistributionAccumulator,
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
pub struct FailureReasonsDistributionAccumulator {
    pub count: u64,
    pub count_without_retries: u64,
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
pub struct ProcessedAmountAccumulator {
    pub count_with_retries: Option<i64>,
    pub total_with_retries: Option<i64>,
    pub count_without_retries: Option<i64>,
    pub total_without_retries: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AverageAccumulator {
    pub total: u32,
    pub count: u32,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct RetriesAmountAccumulator {
    pub total: Option<i64>,
}

#[derive(Debug, Default)]
pub struct PaymentsDistributionAccumulator {
    pub success: u32,
    pub failed: u32,
    pub total: u32,
    pub success_without_retries: u32,
    pub success_with_only_retries: u32,
    pub failed_without_retries: u32,
    pub failed_with_only_retries: u32,
    pub total_without_retries: u32,
    pub total_with_only_retries: u32,
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

impl PaymentMetricAccumulator for FailureReasonsDistributionAccumulator {
    type MetricOutput = (Option<u64>, Option<u64>);

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        if let Some(count) = metrics.count {
            if let Ok(count_u64) = u64::try_from(count) {
                self.count += count_u64;
            }
        }
        if metrics.first_attempt.unwrap_or(false) {
            if let Some(count) = metrics.count {
                if let Ok(count_u64) = u64::try_from(count) {
                    self.count_without_retries += count_u64;
                }
            }
        }
    }

    fn collect(self) -> Self::MetricOutput {
        (Some(self.count), Some(self.count_without_retries))
    }
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

impl PaymentMetricAccumulator for PaymentsDistributionAccumulator {
    type MetricOutput = (
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    );

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::AttemptStatus::Charged {
                if let Some(success) = metrics
                    .count
                    .and_then(|success| u32::try_from(success).ok())
                {
                    self.success += success;
                    if metrics.first_attempt.unwrap_or(false) {
                        self.success_without_retries += success;
                    } else {
                        self.success_with_only_retries += success;
                    }
                }
            }
            if status.as_ref() == &storage_enums::AttemptStatus::Failure {
                if let Some(failed) = metrics.count.and_then(|failed| u32::try_from(failed).ok()) {
                    self.failed += failed;
                    if metrics.first_attempt.unwrap_or(false) {
                        self.failed_without_retries += failed;
                    } else {
                        self.failed_with_only_retries += failed;
                    }
                }
            }
            if status.as_ref() != &storage_enums::AttemptStatus::AuthenticationFailed
                && status.as_ref() != &storage_enums::AttemptStatus::PaymentMethodAwaited
                && status.as_ref() != &storage_enums::AttemptStatus::DeviceDataCollectionPending
                && status.as_ref() != &storage_enums::AttemptStatus::ConfirmationAwaited
                && status.as_ref() != &storage_enums::AttemptStatus::Unresolved
            {
                if let Some(total) = metrics.count.and_then(|total| u32::try_from(total).ok()) {
                    self.total += total;
                    if metrics.first_attempt.unwrap_or(false) {
                        self.total_without_retries += total;
                    } else {
                        self.total_with_only_retries += total;
                    }
                }
            }
        }
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total == 0 {
            (None, None, None, None, None, None)
        } else {
            let success = Some(self.success);
            let success_without_retries = Some(self.success_without_retries);
            let success_with_only_retries = Some(self.success_with_only_retries);
            let failed = Some(self.failed);
            let failed_with_only_retries = Some(self.failed_with_only_retries);
            let failed_without_retries = Some(self.failed_without_retries);
            let total = Some(self.total);
            let total_without_retries = Some(self.total_without_retries);
            let total_with_only_retries = Some(self.total_with_only_retries);

            let success_rate = match (success, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            let success_rate_without_retries =
                match (success_without_retries, total_without_retries) {
                    (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                    _ => None,
                };

            let success_rate_with_only_retries =
                match (success_with_only_retries, total_with_only_retries) {
                    (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                    _ => None,
                };

            let failed_rate = match (failed, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            let failed_rate_without_retries = match (failed_without_retries, total_without_retries)
            {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            let failed_rate_with_only_retries =
                match (failed_with_only_retries, total_with_only_retries) {
                    (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                    _ => None,
                };
            (
                success_rate,
                success_rate_without_retries,
                success_rate_with_only_retries,
                failed_rate,
                failed_rate_without_retries,
                failed_rate_with_only_retries,
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

impl PaymentMetricAccumulator for ProcessedAmountAccumulator {
    type MetricOutput = (
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
    );
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        self.total_with_retries = match (
            self.total_with_retries,
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
        ) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        };

        self.count_with_retries = match (self.count_with_retries, metrics.count) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        };

        if metrics.first_attempt.unwrap_or(false) {
            self.total_without_retries = match (
                self.total_without_retries,
                metrics.total.as_ref().and_then(ToPrimitive::to_i64),
            ) {
                (None, None) => None,
                (None, i @ Some(_)) | (i @ Some(_), None) => i,
                (Some(a), Some(b)) => Some(a + b),
            };

            self.count_without_retries = match (self.count_without_retries, metrics.count) {
                (None, None) => None,
                (None, i @ Some(_)) | (i @ Some(_), None) => i,
                (Some(a), Some(b)) => Some(a + b),
            };
        }
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        let total_with_retries = u64::try_from(self.total_with_retries.unwrap_or(0)).ok();
        let count_with_retries = self.count_with_retries.and_then(|i| u64::try_from(i).ok());

        let total_without_retries = u64::try_from(self.total_without_retries.unwrap_or(0)).ok();
        let count_without_retries = self
            .count_without_retries
            .and_then(|i| u64::try_from(i).ok());

        (
            total_with_retries,
            count_with_retries,
            total_without_retries,
            count_without_retries,
            Some(0),
            Some(0),
        )
    }
}

impl PaymentMetricAccumulator for RetriesAmountAccumulator {
    type MetricOutput = Option<u64>;
    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        self.total = match (
            self.total,
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
        ) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        };
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        u64::try_from(self.total.unwrap_or(0)).ok()
    }
}

impl PaymentMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &PaymentMetricRow) {
        let total = metrics.total.as_ref().and_then(ToPrimitive::to_u32);
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
        let (
            payment_processed_amount,
            payment_processed_count,
            payment_processed_amount_without_smart_retries,
            payment_processed_count_without_smart_retries,
            payment_processed_amount_in_usd,
            payment_processed_amount_without_smart_retries_usd,
        ) = self.processed_amount.collect();
        let (
            payments_success_rate_distribution,
            payments_success_rate_distribution_without_smart_retries,
            payments_success_rate_distribution_with_only_retries,
            payments_failure_rate_distribution,
            payments_failure_rate_distribution_without_smart_retries,
            payments_failure_rate_distribution_with_only_retries,
        ) = self.payments_distribution.collect();
        let (failure_reason_count, failure_reason_count_without_smart_retries) =
            self.failure_reasons_distribution.collect();
        PaymentMetricsBucketValue {
            payment_success_rate: self.payment_success_rate.collect(),
            payment_count: self.payment_count.collect(),
            payment_success_count: self.payment_success.collect(),
            payment_processed_amount,
            payment_processed_count,
            payment_processed_amount_without_smart_retries,
            payment_processed_count_without_smart_retries,
            avg_ticket_size: self.avg_ticket_size.collect(),
            payment_error_message: self.payment_error_message.collect(),
            retries_count: self.retries_count.collect(),
            retries_amount_processed: self.retries_amount_processed.collect(),
            connector_success_rate: self.connector_success_rate.collect(),
            payments_success_rate_distribution,
            payments_success_rate_distribution_without_smart_retries,
            payments_success_rate_distribution_with_only_retries,
            payments_failure_rate_distribution,
            payments_failure_rate_distribution_without_smart_retries,
            payments_failure_rate_distribution_with_only_retries,
            failure_reason_count,
            failure_reason_count_without_smart_retries,
            payment_processed_amount_in_usd,
            payment_processed_amount_without_smart_retries_usd,
        }
    }
}
