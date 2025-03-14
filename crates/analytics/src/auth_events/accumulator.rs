use api_models::analytics::auth_events::AuthEventMetricsBucketValue;

use super::metrics::AuthEventMetricRow;

#[derive(Debug, Default)]
pub struct AuthEventMetricsAccumulator {
    pub authentication_count: CountAccumulator,
    pub authentication_attempt_count: CountAccumulator,
    pub authentication_error_message: AuthenticationErrorMessageAccumulator,
    pub authentication_success_count: CountAccumulator,
    pub challenge_flow_count: CountAccumulator,
    pub challenge_attempt_count: CountAccumulator,
    pub challenge_success_count: CountAccumulator,
    pub frictionless_flow_count: CountAccumulator,
    pub frictionless_success_count: CountAccumulator,
    pub authentication_funnel: CountAccumulator,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct CountAccumulator {
    pub count: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AuthenticationErrorMessageAccumulator {
    pub count: Option<i64>,
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

impl AuthEventMetricAccumulator for AuthenticationErrorMessageAccumulator {
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

impl AuthEventMetricsAccumulator {
    pub fn collect(self) -> AuthEventMetricsBucketValue {
        AuthEventMetricsBucketValue {
            authentication_count: self.authentication_count.collect(),
            authentication_attempt_count: self.authentication_attempt_count.collect(),
            authentication_success_count: self.authentication_success_count.collect(),
            challenge_flow_count: self.challenge_flow_count.collect(),
            challenge_attempt_count: self.challenge_attempt_count.collect(),
            challenge_success_count: self.challenge_success_count.collect(),
            frictionless_flow_count: self.frictionless_flow_count.collect(),
            frictionless_success_count: self.frictionless_success_count.collect(),
            error_message_count: self.authentication_error_message.collect(),
            authentication_funnel: self.authentication_funnel.collect(),
        }
    }
}
