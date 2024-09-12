use api_models::analytics::payment_intents::PaymentIntentMetricsBucketValue;
use bigdecimal::ToPrimitive;
use diesel_models::enums as storage_enums;

use super::metrics::PaymentIntentMetricRow;

#[derive(Debug, Default)]
pub struct PaymentIntentMetricsAccumulator {
    pub successful_smart_retries: CountAccumulator,
    pub total_smart_retries: CountAccumulator,
    pub smart_retried_amount: SumAccumulator,
    pub payment_intent_count: CountAccumulator,
    pub authorization_success_rate: AuthorizationRateAccumulator,
    pub auth_declined_rate: AuthDeclinedRateAccumulator,
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
#[repr(transparent)]
pub struct SumAccumulator {
    pub total: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AuthorizationRateAccumulator {
    pub success: i64,
    pub total: i64,
}

#[derive(Debug, Default)]
pub struct AuthDeclinedRateAccumulator {
    pub failed: i64,
    pub total: i64,
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

impl PaymentIntentMetricAccumulator for SumAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        self.total = match (
            self.total,
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
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

impl PaymentIntentMetricAccumulator for AuthorizationRateAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::IntentStatus::Succeeded {
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

impl PaymentIntentMetricAccumulator for AuthDeclinedRateAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &PaymentIntentMetricRow) {
        if let Some(ref status) = metrics.status {
            if status.as_ref() == &storage_enums::IntentStatus::Failed {
                self.failed += metrics.count.unwrap_or_default();
            }
        };
        self.total += metrics.count.unwrap_or_default();
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total <= 0 {
            None
        } else {
            Some(
                f64::from(u32::try_from(self.failed).ok()?) * 100.0
                    / f64::from(u32::try_from(self.total).ok()?),
            )
        }
    }
}

impl PaymentIntentMetricsAccumulator {
    pub fn collect(self) -> PaymentIntentMetricsBucketValue {
        PaymentIntentMetricsBucketValue {
            successful_smart_retries: self.successful_smart_retries.collect(),
            total_smart_retries: self.total_smart_retries.collect(),
            smart_retried_amount: self.smart_retried_amount.collect(),
            payment_intent_count: self.payment_intent_count.collect(),
            authorization_success_rate: self.authorization_success_rate.collect(),
            auth_declined_rate: self.auth_declined_rate.collect(),
        }
    }
}
