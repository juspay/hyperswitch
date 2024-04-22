use api_models::analytics::auth_events::AuthEventMetricsBucketValue;
use router_env::logger;

use super::metrics::AuthEventMetricRow;

#[derive(Debug, Default)]
pub struct AuthEventMetricsAccumulator {
    pub three_ds_sdk_count: CountAccumulator,
    pub authentication_attempt_count: CountAccumulator,
    pub authentication_success_count: CountAccumulator,
    pub challenge_flow_count: CountAccumulator,
    pub challenge_attempt_count: CountAccumulator,
    pub challenge_success_count: CountAccumulator,
    pub frictionless_flow_count: CountAccumulator,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AverageAccumulator {
    pub total: u32,
    pub count: u32,
}

pub trait AuthEventMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &AuthEventMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl AuthEventMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &AuthEventMetricRow) {
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

impl AuthEventMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &AuthEventMetricRow) {
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

impl AuthEventMetricsAccumulator {
    #[allow(dead_code)]
    pub fn collect(self) -> AuthEventMetricsBucketValue {
        AuthEventMetricsBucketValue {
            three_ds_sdk_count: self.three_ds_sdk_count.collect(),
            authentication_attempt_count: self.authentication_attempt_count.collect(),
            authentication_success_count: self.authentication_success_count.collect(),
            challenge_flow_count: self.challenge_flow_count.collect(),
            challenge_attempt_count: self.challenge_attempt_count.collect(),
            challenge_success_count: self.challenge_success_count.collect(),
            frictionless_flow_count: self.frictionless_flow_count.collect(),
        }
    }
}
