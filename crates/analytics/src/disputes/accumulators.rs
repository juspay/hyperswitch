use api_models::analytics::disputes::DisputeMetricsBucketValue;
use diesel_models::enums as storage_enums;

use super::metrics::DisputeMetricRow;
#[derive(Debug, Default)]
pub struct DisputeMetricsAccumulator {
    pub disputes_status_rate: RateAccumulator,
    pub total_amount_disputed: SumAccumulator,
    pub total_dispute_lost_amount: SumAccumulator,
}
#[derive(Debug, Default)]
pub struct RateAccumulator {
    pub won_count: i64,
    pub challenged_count: i64,
    pub lost_count: i64,
    pub total: i64,
}
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct SumAccumulator {
    pub total: Option<i64>,
}

pub trait DisputeMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &DisputeMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl DisputeMetricAccumulator for SumAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &DisputeMetricRow) {
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

impl DisputeMetricAccumulator for RateAccumulator {
    type MetricOutput = Option<(Option<u64>, Option<u64>, Option<u64>, Option<u64>)>;

    fn add_metrics_bucket(&mut self, metrics: &DisputeMetricRow) {
        if let Some(ref dispute_status) = metrics.dispute_status {
            if dispute_status.as_ref() == &storage_enums::DisputeStatus::DisputeChallenged {
                self.challenged_count += metrics.count.unwrap_or_default();
            }
            if dispute_status.as_ref() == &storage_enums::DisputeStatus::DisputeWon {
                self.won_count += metrics.count.unwrap_or_default();
            }
            if dispute_status.as_ref() == &storage_enums::DisputeStatus::DisputeLost {
                self.lost_count += metrics.count.unwrap_or_default();
            }
        };

        self.total += metrics.count.unwrap_or_default();
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total <= 0 {
            Some((None, None, None, None))
        } else {
            Some((
                u64::try_from(self.challenged_count).ok(),
                u64::try_from(self.won_count).ok(),
                u64::try_from(self.lost_count).ok(),
                u64::try_from(self.total).ok(),
            ))
        }
    }
}

impl DisputeMetricsAccumulator {
    pub fn collect(self) -> DisputeMetricsBucketValue {
        let (challenge_rate, won_rate, lost_rate, total_dispute) =
            self.disputes_status_rate.collect().unwrap_or_default();
        DisputeMetricsBucketValue {
            disputes_challenged: challenge_rate,
            disputes_won: won_rate,
            disputes_lost: lost_rate,
            total_amount_disputed: self.total_amount_disputed.collect(),
            total_dispute_lost_amount: self.total_dispute_lost_amount.collect(),
            total_dispute: total_dispute,
        }
    }
}
