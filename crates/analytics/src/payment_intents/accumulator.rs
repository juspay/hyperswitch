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
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        self.amount = match (
            self.amount,
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
        ) {
            (None, None) => None,
            (None, i @ Some(_)) | (i @ Some(_), None) => i,
            (Some(a), Some(b)) => Some(a + b),
        }
    }
    #[inline]
    fn collect(self) -> Self::MetricOutput {
        self.amount.and_then(|i| u64::try_from(i).ok())
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

impl PaymentIntentMetricsAccumulator {
    pub fn collect(self) -> PaymentIntentMetricsBucketValue {
        let (
            successful_payments,
            successful_payments_without_smart_retries,
            total_payments,
            payments_success_rate,
            payments_success_rate_without_smart_retries,
        ) = self.payments_success_rate.collect();
        PaymentIntentMetricsBucketValue {
            successful_smart_retries: self.successful_smart_retries.collect(),
            total_smart_retries: self.total_smart_retries.collect(),
            smart_retried_amount: self.smart_retried_amount.collect(),
            payment_intent_count: self.payment_intent_count.collect(),
            successful_payments,
            successful_payments_without_smart_retries,
            total_payments,
            payments_success_rate,
            payments_success_rate_without_smart_retries,
        }
    }
}
