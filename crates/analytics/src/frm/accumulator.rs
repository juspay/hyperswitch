use api_models::analytics::frm::FrmMetricsBucketValue;
use common_enums::enums as storage_enums;

use super::metrics::FrmMetricRow;
#[derive(Debug, Default)]
pub struct FrmMetricsAccumulator {
    pub frm_triggered_attempts: TriggeredAttemptsAccumulator,
    pub frm_blocked_rate: BlockedRateAccumulator,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct TriggeredAttemptsAccumulator {
    pub count: Option<i64>,
}

#[derive(Debug, Default)]
pub struct BlockedRateAccumulator {
    pub fraud: i64,
    pub total: i64,
}

pub trait FrmMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &FrmMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl FrmMetricAccumulator for TriggeredAttemptsAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &FrmMetricRow) {
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

impl FrmMetricAccumulator for BlockedRateAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &FrmMetricRow) {
        if let Some(ref frm_status) = metrics.frm_status {
            if frm_status.as_ref() == &storage_enums::FraudCheckStatus::Fraud {
                self.fraud += metrics.count.unwrap_or_default();
            }
        };
        self.total += metrics.count.unwrap_or_default();
    }

    fn collect(self) -> Self::MetricOutput {
        if self.total <= 0 {
            None
        } else {
            Some(
                f64::from(u32::try_from(self.fraud).ok()?) * 100.0
                    / f64::from(u32::try_from(self.total).ok()?),
            )
        }
    }
}

impl FrmMetricsAccumulator {
    pub fn collect(self) -> FrmMetricsBucketValue {
        FrmMetricsBucketValue {
            frm_blocked_rate: self.frm_blocked_rate.collect(),
            frm_triggered_attempts: self.frm_triggered_attempts.collect(),
        }
    }
}
