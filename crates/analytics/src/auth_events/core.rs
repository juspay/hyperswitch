use std::collections::HashMap;

use api_models::analytics::{
    auth_events::{AuthEventMetrics, AuthEventMetricsBucketIdentifier, MetricsBucketResponse},
    AnalyticsMetadata, GetAuthEventMetricRequest, MetricsResponse,
};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use super::AuthEventMetricsAccumulator;
use crate::{
    auth_events::AuthEventMetricAccumulator,
    errors::{AnalyticsError, AnalyticsResult},
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &String,
    publishable_key: Option<&String>,
    req: GetAuthEventMetricRequest,
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        AuthEventMetricsBucketIdentifier,
        AuthEventMetricsAccumulator,
    > = HashMap::new();

    if let Some(publishable_key) = publishable_key {
        let mut set = tokio::task::JoinSet::new();
        for metric_type in req.metrics.iter().cloned() {
            let req = req.clone();
            let merchant_id_scoped = merchant_id.to_owned();
            let publishable_key_scoped = publishable_key.to_owned();
            let pool = pool.clone();
            set.spawn(async move {
                let data = pool
                    .get_auth_event_metrics(
                        &metric_type,
                        &merchant_id_scoped,
                        &publishable_key_scoped,
                        &req.time_series.map(|t| t.granularity),
                        &req.time_range,
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                (metric_type, data)
            });
        }

        while let Some((metric, data)) = set
            .join_next()
            .await
            .transpose()
            .change_context(AnalyticsError::UnknownError)?
        {
            logger::info!("Logging Result {:?}", data);
            for (id, value) in data? {
                let metrics_builder = metrics_accumulator.entry(id).or_default();
                match metric {
                    AuthEventMetrics::ThreeDsSdkCount => metrics_builder
                        .three_ds_sdk_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::AuthenticationAttemptCount => metrics_builder
                        .authentication_attempt_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::AuthenticationSuccessCount => metrics_builder
                        .authentication_success_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::ChallengeFlowCount => metrics_builder
                        .challenge_flow_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::ChallengeAttemptCount => metrics_builder
                        .challenge_attempt_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::ChallengeSuccessCount => metrics_builder
                        .challenge_success_count
                        .add_metrics_bucket(&value),
                    AuthEventMetrics::FrictionlessFlowCount => metrics_builder
                        .frictionless_flow_count
                        .add_metrics_bucket(&value),
                }
            }

            logger::debug!(
                "Analytics Accumulated Results: metric: {}, results: {:#?}",
                metric,
                metrics_accumulator
            );
        }

        let query_data: Vec<MetricsBucketResponse> = metrics_accumulator
            .into_iter()
            .map(|(id, val)| MetricsBucketResponse {
                values: val.collect(),
                dimensions: id,
            })
            .collect();

        Ok(MetricsResponse {
            query_data,
            meta_data: [AnalyticsMetadata {
                current_time_range: req.time_range,
            }],
        })
    } else {
        logger::error!("Publishable key not present for merchant ID");
        Ok(MetricsResponse {
            query_data: vec![],
            meta_data: [AnalyticsMetadata {
                current_time_range: req.time_range,
            }],
        })
    }
}
