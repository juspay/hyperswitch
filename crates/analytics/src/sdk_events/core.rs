use std::collections::HashMap;

use api_models::analytics::{
    sdk_events::{
        MetricsBucketResponse, SdkEventMetrics, SdkEventMetricsBucketIdentifier, SdkEventsRequest,
    },
    AnalyticsMetadata, GetSdkEventFiltersRequest, GetSdkEventMetricRequest, MetricsResponse,
    SdkEventFiltersResponse,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, logger, tracing};

use super::{
    events::{get_sdk_event, SdkEventsResult},
    SdkEventMetricsAccumulator,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    sdk_events::SdkEventMetricAccumulator,
    types::FiltersError,
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn sdk_events_core(
    pool: &AnalyticsProvider,
    req: SdkEventsRequest,
    publishable_key: String,
) -> AnalyticsResult<Vec<SdkEventsResult>> {
    match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "SDK Events not implemented for SQLX",
        ))
        .into_report()
        .attach_printable("SQL Analytics is not implemented for Sdk Events"),
        AnalyticsProvider::Clickhouse(pool) => get_sdk_event(&publishable_key, req, pool).await,
        AnalyticsProvider::CombinedSqlx(_sqlx_pool, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_sqlx_pool, ckh_pool) => {
            get_sdk_event(&publishable_key, req, ckh_pool).await
        }
    }
    .switch()
}

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    publishable_key: Option<&String>,
    req: GetSdkEventMetricRequest,
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        SdkEventMetricsBucketIdentifier,
        SdkEventMetricsAccumulator,
    > = HashMap::new();

    if let Some(publishable_key) = publishable_key {
        let mut set = tokio::task::JoinSet::new();
        for metric_type in req.metrics.iter().cloned() {
            let req = req.clone();
            let publishable_key_scoped = publishable_key.to_owned();
            let pool = pool.clone();
            set.spawn(async move {
                let data = pool
                    .get_sdk_event_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &publishable_key_scoped,
                        &req.filters,
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
            .into_report()
            .change_context(AnalyticsError::UnknownError)?
        {
            logger::info!("Logging Result {:?}", data);
            for (id, value) in data? {
                let metrics_builder = metrics_accumulator.entry(id).or_default();
                match metric {
                    SdkEventMetrics::PaymentAttempts => {
                        metrics_builder.payment_attempts.add_metrics_bucket(&value)
                    }
                    SdkEventMetrics::PaymentMethodsCallCount => metrics_builder
                        .payment_methods_call_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::SdkRenderedCount => metrics_builder
                        .sdk_rendered_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::SdkInitiatedCount => metrics_builder
                        .sdk_initiated_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::PaymentMethodSelectedCount => metrics_builder
                        .payment_method_selected_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::PaymentDataFilledCount => metrics_builder
                        .payment_data_filled_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::AveragePaymentTime => metrics_builder
                        .average_payment_time
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsMethodInvokedCount => metrics_builder
                        .three_ds_method_invoked_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsMethodSkippedCount => metrics_builder
                        .three_ds_method_skipped_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsMethodSuccessfulCount => metrics_builder
                        .three_ds_method_successful_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsMethodUnsuccessfulCount => metrics_builder
                        .three_ds_method_unsuccessful_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::AuthenticationUnsuccessfulCount => metrics_builder
                        .authentication_unsuccessful_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsChallengeFlowCount => metrics_builder
                        .three_ds_challenge_flow_count
                        .add_metrics_bucket(&value),
                    SdkEventMetrics::ThreeDsFrictionlessFlowCount => metrics_builder
                        .three_ds_frictionless_flow_count
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

#[allow(dead_code)]
pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetSdkEventFiltersRequest,
    publishable_key: Option<&String>,
) -> AnalyticsResult<SdkEventFiltersResponse> {
    use api_models::analytics::{sdk_events::SdkEventDimensions, SdkEventFilterValue};

    use super::filters::get_sdk_event_filter_for_dimension;
    use crate::sdk_events::filters::SdkEventFilter;

    let mut res = SdkEventFiltersResponse::default();

    if let Some(publishable_key) = publishable_key {
        for dim in req.group_by_names {
            let values = match pool {
                AnalyticsProvider::Sqlx(_pool) => Err(FiltersError::NotImplemented(
                    "SDK Events not implemented for SQLX",
                ))
                .into_report()
                .attach_printable("SQL Analytics is not implemented for SDK Events"),
                AnalyticsProvider::Clickhouse(pool) => {
                    get_sdk_event_filter_for_dimension(dim, publishable_key, &req.time_range, pool)
                        .await
                }
                AnalyticsProvider::CombinedSqlx(_sqlx_pool, ckh_pool)
                | AnalyticsProvider::CombinedCkh(_sqlx_pool, ckh_pool) => {
                    get_sdk_event_filter_for_dimension(
                        dim,
                        publishable_key,
                        &req.time_range,
                        ckh_pool,
                    )
                    .await
                }
            }
            .change_context(AnalyticsError::UnknownError)?
            .into_iter()
            .filter_map(|fil: SdkEventFilter| match dim {
                SdkEventDimensions::PaymentMethod => fil.payment_method,
                SdkEventDimensions::Platform => fil.platform,
                SdkEventDimensions::BrowserName => fil.browser_name,
                SdkEventDimensions::Source => fil.source,
                SdkEventDimensions::Component => fil.component,
                SdkEventDimensions::PaymentExperience => fil.payment_experience,
            })
            .collect::<Vec<String>>();
            res.query_data.push(SdkEventFilterValue {
                dimension: dim,
                values,
            })
        }
    } else {
        router_env::logger::error!("Publishable key not found for merchant");
    }

    Ok(res)
}
