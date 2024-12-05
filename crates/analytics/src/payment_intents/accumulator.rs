use api_models::analytics::payment_intents::PaymentIntentMetricsBucketValue;
use bigdecimal::ToPrimitive;
use diesel_models::enums as storage_enums;

use super::metrics::PaymentIntentMetricRow;

#[derive(Debug, Default)]
pub struct PaymentIntentMetricsAccumulator {
    pub successful_smart_retries: CountAccumulator,
    pub total_smart_retries: CountAccumulator,
    pub smart_retried_amount: SmartRetriedAmountAccumulator,
    pub payment_intent_count: CountAccumulator,
    pub payments_success_rate: PaymentsSuccessRateAccumulator,
    pub payment_processed_amount: ProcessedAmountAccumulator,
    pub payments_distribution: PaymentsDistributionAccumulator,
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
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}

pub trait PaymentIntentMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

#[derive(Debug, Default)]
pub struct SmartRetriedAmountAccumulator {
    pub amount: Option<i64>,
    pub amount_without_retries: Option<i64>,
}

#[derive(Debug, Default)]
pub struct PaymentsSuccessRateAccumulator {
    pub success: u32,
    pub success_without_retries: u32,
    pub total: u32,
}

#[derive(Debug, Default)]
pub struct ProcessedAmountAccumulator {
    pub count_with_retries: Option<i64>,
    pub total_with_retries: Option<i64>,
    pub count_without_retries: Option<i64>,
    pub total_without_retries: Option<i64>,
}

#[derive(Debug, Default)]
pub struct PaymentsDistributionAccumulator {
    pub success_without_retries: u32,
    pub failed_without_retries: u32,
    pub total: u32,
}

impl PaymentIntentMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
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

impl PaymentIntentMetricAccumulator for SmartRetriedAmountAccumulator {
    type MetricOutput = (Option<u64>, Option<u64>, Option<u64>, Option<u64>);
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        self.amount = match (
            self.amount,
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
        ) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        };
        if metrics.first_attempt.unwrap_or(0) == 1 {
            self.amount_without_retries = match (
                self.amount_without_retries,
                metrics.total.as_ref().and_then(ToPrimitive::to_i64),
            ) {
                (None, None) => None,
                (None, i @ Some(_)) | (i @ Some(_), None) => i,
                (Some(a), Some(b)) => Some(a + b),
            }
        } else {
            self.amount_without_retries = Some(0);
        }
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        let with_retries = self.amount.and_then(|i| u64::try_from(i).ok()).or(Some(0));
        let without_retries = self
            .amount_without_retries
            .and_then(|i| u64::try_from(i).ok())
            .or(Some(0));
        (with_retries, without_retries, Some(0), Some(0))
    }
}

impl PaymentIntentMetricAccumulator for PaymentsSuccessRateAccumulator {
    type MetricOutput = (
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<f64>,
        Option<f64>,
    );

    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::IntentStatus::Succeeded {
                if let Some(success) = metrics
                    .count
                    .and_then(|success| u32::try_from(success).ok())
                {
                    self.success += success;
                    if metrics.first_attempt.unwrap_or(0) == 1 {
                        self.success_without_retries += success;
                    }
                }
            }
            if status.as_ref() != &storage_enums::IntentStatus::RequiresCustomerAction
                && status.as_ref() != &storage_enums::IntentStatus::RequiresPaymentMethod
                && status.as_ref() != &storage_enums::IntentStatus::RequiresMerchantAction
                && status.as_ref() != &storage_enums::IntentStatus::RequiresConfirmation
            {
                if let Some(total) = metrics.count.and_then(|total| u32::try_from(total).ok()) {
                    self.total += total;
                }
            }
        }
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total == 0 {
            (None, None, None, None, None)
        } else {
            let success = Some(self.success);
            let success_without_retries = Some(self.success_without_retries);
            let total = Some(self.total);

            let success_rate = match (success, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            let success_without_retries_rate = match (success_without_retries, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            (
                success,
                success_without_retries,
                total,
                success_rate,
                success_without_retries_rate,
            )
        }
    }
}

impl PaymentIntentMetricAccumulator for ProcessedAmountAccumulator {
    type MetricOutput = (
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
    );
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
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

        if metrics.first_attempt.unwrap_or(0) == 1 {
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

impl PaymentIntentMetricAccumulator for PaymentsDistributionAccumulator {
    type MetricOutput = (Option<f64>, Option<f64>);

    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        let first_attempt = metrics.first_attempt.unwrap_or(0);
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::IntentStatus::Succeeded {
                if let Some(success) = metrics
                    .count
                    .and_then(|success| u32::try_from(success).ok())
                {
                    if first_attempt == 1 {
                        self.success_without_retries += success;
                    }
                }
            }
            if let Some(failed) = metrics.count.and_then(|failed| u32::try_from(failed).ok()) {
                if first_attempt == 0
                    || (first_attempt == 1
                        && status.as_ref() == &storage_enums::IntentStatus::Failed)
                {
                    self.failed_without_retries += failed;
                }
            }

            if status.as_ref() != &storage_enums::IntentStatus::RequiresCustomerAction
                && status.as_ref() != &storage_enums::IntentStatus::RequiresPaymentMethod
                && status.as_ref() != &storage_enums::IntentStatus::RequiresMerchantAction
                && status.as_ref() != &storage_enums::IntentStatus::RequiresConfirmation
            {
                if let Some(total) = metrics.count.and_then(|total| u32::try_from(total).ok()) {
                    self.total += total;
                }
            }
        }
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total == 0 {
            (None, None)
        } else {
            let success_without_retries = Some(self.success_without_retries);
            let failed_without_retries = Some(self.failed_without_retries);
            let total = Some(self.total);

            let success_rate_without_retries = match (success_without_retries, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };

            let failed_rate_without_retries = match (failed_without_retries, total) {
                (Some(s), Some(t)) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
                _ => None,
            };
            (success_rate_without_retries, failed_rate_without_retries)
        }
    }
}

impl PaymentIntentMetricsAccumulator {
    pub fn collect(self) -> PaymentIntentMetricsBucketValue {
        let (
            successful_payments,
            successful_payments_without_smart_retries,
            total_payments,
            payments_success_rate,
            payments_success_rate_without_smart_retries,
        ) = self.payments_success_rate.collect();
        let (
            smart_retried_amount,
            smart_retried_amount_without_smart_retries,
            smart_retried_amount_in_usd,
            smart_retried_amount_without_smart_retries_in_usd,
        ) = self.smart_retried_amount.collect();
        let (
            payment_processed_amount,
            payment_processed_count,
            payment_processed_amount_without_smart_retries,
            payment_processed_count_without_smart_retries,
            payment_processed_amount_in_usd,
            payment_processed_amount_without_smart_retries_in_usd,
        ) = self.payment_processed_amount.collect();
        let (
            payments_success_rate_distribution_without_smart_retries,
            payments_failure_rate_distribution_without_smart_retries,
        ) = self.payments_distribution.collect();
        PaymentIntentMetricsBucketValue {
            successful_smart_retries: self.successful_smart_retries.collect(),
            total_smart_retries: self.total_smart_retries.collect(),
            smart_retried_amount,
            smart_retried_amount_in_usd,
            smart_retried_amount_without_smart_retries,
            smart_retried_amount_without_smart_retries_in_usd,
            payment_intent_count: self.payment_intent_count.collect(),
            successful_payments,
            successful_payments_without_smart_retries,
            total_payments,
            payments_success_rate,
            payments_success_rate_without_smart_retries,
            payment_processed_amount,
            payment_processed_count,
            payment_processed_amount_without_smart_retries,
            payment_processed_count_without_smart_retries,
            payments_success_rate_distribution_without_smart_retries,
            payments_failure_rate_distribution_without_smart_retries,
            payment_processed_amount_in_usd,
            payment_processed_amount_without_smart_retries_in_usd,
        }
    }
}
