use std::collections::HashMap;

use api_models::analytics::{
    auth_events::{
        AuthEventDimensions, AuthEventMetrics, AuthEventMetricsBucketIdentifier,
        MetricsBucketResponse,
    },
    AuthEventFilterValue, AuthEventFiltersResponse, AuthEventMetricsResponse,
    AuthEventsAnalyticsMetadata, GetAuthEventFilterRequest, GetAuthEventMetricRequest,
};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{
    filters::{get_auth_events_filter_for_dimension, AuthEventFilterRow},
    AuthEventMetricsAccumulator,
};
use crate::{
    auth_events::AuthEventMetricAccumulator,
    errors::{AnalyticsError, AnalyticsResult},
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &common_utils::id_type::MerchantId,
    req: GetAuthEventMetricRequest,
) -> AnalyticsResult<AuthEventMetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        AuthEventMetricsBucketIdentifier,
        AuthEventMetricsAccumulator,
    > = HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let merchant_id_scoped = merchant_id.to_owned();
        let pool = pool.clone();
        set.spawn(async move {
            let data = pool
                .get_auth_event_metrics(
                    &metric_type,
                    &req.group_by_names.clone(),
                    &merchant_id_scoped,
                    &req.filters,
                    req.time_series.map(|t| t.granularity),
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
        for (id, value) in data? {
            let metrics_builder = metrics_accumulator.entry(id).or_default();
            match metric {
                AuthEventMetrics::AuthenticationCount => metrics_builder
                    .authentication_count
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
                AuthEventMetrics::FrictionlessSuccessCount => metrics_builder
                    .frictionless_success_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationErrorMessage => metrics_builder
                    .authentication_error_message
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationFunnel => metrics_builder
                    .authentication_funnel
                    .add_metrics_bucket(&value),
            }
        }
    }

    let mut total_error_message_count = 0;
    let query_data: Vec<MetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let collected_values = val.collect();
            if let Some(count) = collected_values.error_message_count {
                total_error_message_count += count;
            }
            MetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();
    Ok(AuthEventMetricsResponse {
        query_data,
        meta_data: [AuthEventsAnalyticsMetadata {
            total_error_message_count: Some(total_error_message_count),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetAuthEventFilterRequest,
    merchant_id: &common_utils::id_type::MerchantId,
) -> AnalyticsResult<AuthEventFiltersResponse> {
    let mut res = AuthEventFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(_pool) => {
                            Err(report!(AnalyticsError::UnknownError))
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_auth_events_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
                    .map_err(|e| e.change_context(AnalyticsError::UnknownError))
            }
                    AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) | AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_auth_events_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await
                .map_err(|e| e.change_context(AnalyticsError::UnknownError));
                let sqlx_result = get_auth_events_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_pool,
                )
                .await
                .map_err(|e| e.change_context(AnalyticsError::UnknownError));
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: AuthEventFilterRow| match dim {
            AuthEventDimensions::AuthenticationStatus => fil.authentication_status.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::TransactionStatus => fil.trans_status.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::AuthenticationType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::ErrorMessage => fil.error_message,
            AuthEventDimensions::AuthenticationConnector => fil.authentication_connector.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::MessageVersion => fil.message_version,
            AuthEventDimensions::AcsReferenceNumber => fil.acs_reference_number,
        })
        .collect::<Vec<String>>();
        res.query_data.push(AuthEventFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
