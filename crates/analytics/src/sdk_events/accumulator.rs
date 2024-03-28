use api_models::analytics::sdk_events::SdkEventMetricsBucketValue;
use router_env::logger;

use super::metrics::SdkEventMetricRow;

#[derive(Debug, Default)]
pub struct SdkEventMetricsAccumulator {
    pub payment_attempts: CountAccumulator,
    pub payment_methods_call_count: CountAccumulator,
    pub average_payment_time: AverageAccumulator,
    pub sdk_initiated_count: CountAccumulator,
    pub sdk_rendered_count: CountAccumulator,
    pub payment_method_selected_count: CountAccumulator,
    pub payment_data_filled_count: CountAccumulator,
    pub three_ds_method_invoked_count: CountAccumulator,
    pub three_ds_method_skipped_count: CountAccumulator,
    pub three_ds_method_successful_count: CountAccumulator,
    pub three_ds_method_unsuccessful_count: CountAccumulator,
    pub authentication_unsuccessful_count: CountAccumulator,
    pub three_ds_challenge_flow_count: CountAccumulator,
    pub three_ds_frictionless_flow_count: CountAccumulator,
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

pub trait SdkEventMetricAccumulator {
    type MetricOutput;

    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow);

    fn collect(self) -> Self::MetricOutput;
}

impl SdkEventMetricAccumulator for CountAccumulator {
    type MetricOutput = Option<u64>;
    #[inline]
    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow) {
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

impl SdkEventMetricAccumulator for AverageAccumulator {
    type MetricOutput = Option<f64>;

    fn add_metrics_bucket(&mut self, metrics: &SdkEventMetricRow) {
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

impl SdkEventMetricsAccumulator {
    #[allow(dead_code)]
    pub fn collect(self) -> SdkEventMetricsBucketValue {
        SdkEventMetricsBucketValue {
            payment_attempts: self.payment_attempts.collect(),
            payment_methods_call_count: self.payment_methods_call_count.collect(),
            average_payment_time: self.average_payment_time.collect(),
            sdk_initiated_count: self.sdk_initiated_count.collect(),
            sdk_rendered_count: self.sdk_rendered_count.collect(),
            payment_method_selected_count: self.payment_method_selected_count.collect(),
            payment_data_filled_count: self.payment_data_filled_count.collect(),
            three_ds_method_invoked_count: self.three_ds_method_invoked_count.collect(),
            three_ds_method_skipped_count: self.three_ds_method_skipped_count.collect(),
            three_ds_method_successful_count: self.three_ds_method_successful_count.collect(),
            three_ds_method_unsuccessful_count: self.three_ds_method_unsuccessful_count.collect(),
            authentication_unsuccessful_count: self.authentication_unsuccessful_count.collect(),
            three_ds_challenge_flow_count: self.three_ds_challenge_flow_count.collect(),
            three_ds_frictionless_flow_count: self.three_ds_frictionless_flow_count.collect(),
        }
    }
}
