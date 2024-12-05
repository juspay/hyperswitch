use api_models::analytics::refunds::{
    ErrorMessagesResult, ReasonsResult, RefundMetricsBucketValue,
};
use bigdecimal::ToPrimitive;
use diesel_models::enums as storage_enums;

use super::{distribution::RefundDistributionRow, metrics::RefundMetricRow};
#[derive(Debug, Default)]
pub struct RefundMetricsAccumulator {
    pub refund_success_rate: SuccessRateAccumulator,
    pub refund_count: CountAccumulator,
    pub refund_success: CountAccumulator,
    pub processed_amount: RefundProcessedAmountAccumulator,
    pub refund_reason: RefundReasonAccumulator,
    pub refund_reason_distribution: RefundReasonDistributionAccumulator,
    pub refund_error_message: RefundReasonAccumulator,
    pub refund_error_message_distribution: RefundErrorMessageDistributionAccumulator,
}

#[derive(Debug, Default)]
pub struct RefundReasonDistributionRow {
    pub count: i64,
    pub total: i64,
    pub refund_reason: String,
}

#[derive(Debug, Default)]
pub struct RefundReasonDistributionAccumulator {
    pub refund_reason_vec: Vec<RefundReasonDistributionRow>,
}

#[derive(Debug, Default)]
pub struct RefundErrorMessageDistributionRow {
    pub count: i64,
    pub total: i64,
    pub refund_error_message: String,
}

#[derive(Debug, Default)]
pub struct RefundErrorMessageDistributionAccumulator {
    pub refund_error_message_vec: Vec<RefundErrorMessageDistributionRow>,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct RefundReasonAccumulator {
    pub count: u64,
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

pub trait RefundDistributionAccumulator {
    type DistributionOutput;

    fn add_distribution_bucket(&mut self, distribution: &RefundDistributionRow);

    fn collect(self) -> Self::DistributionOutput;
}

impl RefundDistributionAccumulator for RefundReasonDistributionAccumulator {
    type DistributionOutput = Option<Vec<ReasonsResult>>;

    fn add_distribution_bucket(&mut self, distribution: &RefundDistributionRow) {
        self.refund_reason_vec.push(RefundReasonDistributionRow {
            count: distribution.count.unwrap_or_default(),
            total: distribution
                .total
                .clone()
                .map(|i| i.to_i64().unwrap_or_default())
                .unwrap_or_default(),
            refund_reason: distribution.refund_reason.clone().unwrap_or_default(),
        })
    }

    fn collect(mut self) -> Self::DistributionOutput {
        if self.refund_reason_vec.is_empty() {
            None
        } else {
            self.refund_reason_vec.sort_by(|a, b| b.count.cmp(&a.count));
            let mut res: Vec<ReasonsResult> = Vec::new();
            for val in self.refund_reason_vec.into_iter() {
                let perc = f64::from(u32::try_from(val.count).ok()?) * 100.0
                    / f64::from(u32::try_from(val.total).ok()?);

                res.push(ReasonsResult {
                    reason: val.refund_reason,
                    count: val.count,
                    percentage: (perc * 100.0).round() / 100.0,
                })
            }

            Some(res)
        }
    }
}

impl RefundDistributionAccumulator for RefundErrorMessageDistributionAccumulator {
    type DistributionOutput = Option<Vec<ErrorMessagesResult>>;

    fn add_distribution_bucket(&mut self, distribution: &RefundDistributionRow) {
        self.refund_error_message_vec
            .push(RefundErrorMessageDistributionRow {
                count: distribution.count.unwrap_or_default(),
                total: distribution
                    .total
                    .clone()
                    .map(|i| i.to_i64().unwrap_or_default())
                    .unwrap_or_default(),
                refund_error_message: distribution
                    .refund_error_message
                    .clone()
                    .unwrap_or_default(),
            })
    }

    fn collect(mut self) -> Self::DistributionOutput {
        if self.refund_error_message_vec.is_empty() {
            None
        } else {
            self.refund_error_message_vec
                .sort_by(|a, b| b.count.cmp(&a.count));
            let mut res: Vec<ErrorMessagesResult> = Vec::new();
            for val in self.refund_error_message_vec.into_iter() {
                let perc = f64::from(u32::try_from(val.count).ok()?) * 100.0
                    / f64::from(u32::try_from(val.total).ok()?);

                res.push(ErrorMessagesResult {
                    error_message: val.refund_error_message,
                    count: val.count,
                    percentage: (perc * 100.0).round() / 100.0,
                })
            }

            Some(res)
        }
    }
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
            metrics.total.as_ref().and_then(ToPrimitive::to_i64),
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
        let total = u64::try_from(self.total.unwrap_or_default()).ok();
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

impl RefundMetricAccumulator for RefundReasonAccumulator {
    type MetricOutput = Option<u64>;

    fn add_metrics_bucket(&mut self, metrics: &RefundMetricRow) {
        if let Some(count) = metrics.count {
            if let Ok(count_u64) = u64::try_from(count) {
                self.count += count_u64;
            }
        }
    }

    fn collect(self) -> Self::MetricOutput {
        Some(self.count)
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
            refund_reason_distribution: self.refund_reason_distribution.collect(),
            refund_error_message_distribution: self.refund_error_message_distribution.collect(),
            refund_reason_count: self.refund_reason.collect(),
            refund_error_message_count: self.refund_error_message.collect(),
        }
    }
}
