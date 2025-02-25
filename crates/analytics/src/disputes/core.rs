use std::collections::HashMap;

use api_models::analytics::{
    disputes::{
        DisputeDimensions, DisputeMetrics, DisputeMetricsBucketIdentifier,
        DisputeMetricsBucketResponse,
    },
    DisputeFilterValue, DisputeFiltersResponse, DisputesAnalyticsMetadata, DisputesMetricsResponse,
    GetDisputeFilterRequest, GetDisputeMetricRequest,
};
use error_stack::ResultExt;
use router_env::{
    logger,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_dispute_filter_for_dimension, DisputeFilterRow},
    DisputeMetricsAccumulator,
};
use crate::{
    disputes::DisputeMetricAccumulator,
    enums::AuthInfo,
    errors::{AnalyticsError, AnalyticsResult},
    metrics, AnalyticsProvider,
};

pub async fn get_metrics(
    pool: &AnalyticsProvider,
    auth: &AuthInfo,
    req: GetDisputeMetricRequest,
) -> AnalyticsResult<DisputesMetricsResponse<DisputeMetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        DisputeMetricsBucketIdentifier,
        DisputeMetricsAccumulator,
    > = HashMap::new();
    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_dispute_query",
            refund_metric = metric_type.as_ref()
        );
        // Currently JoinSet works with only static lifetime references even if the task pool does not outlive the given reference
        // We can optimize away this clone once that is fixed
        let auth_scoped = auth.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_dispute_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &auth_scoped,
                        &req.filters,
                        req.time_series.map(|t| t.granularity),
                        &req.time_range,
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                (metric_type, data)
            }
            .instrument(task_span),
        );
    }

    while let Some((metric, data)) = set
        .join_next()
        .await
        .transpose()
        .change_context(AnalyticsError::UnknownError)?
    {
        let data = data?;
        let attributes = router_env::metric_attributes!(
            ("metric_type", metric.to_string()),
            ("source", pool.to_string()),
        );

        let value = u64::try_from(data.len());
        if let Ok(val) = value {
            metrics::BUCKETS_FETCHED.record(val, attributes);
            logger::debug!("Attributes: {:?}, Buckets fetched: {}", attributes, val);
        }

        for (id, value) in data {
            logger::debug!(bucket_id=?id, bucket_value=?value, "Bucket row for metric {metric}");
            let metrics_builder = metrics_accumulator.entry(id).or_default();
            match metric {
                DisputeMetrics::DisputeStatusMetric
                | DisputeMetrics::SessionizedDisputeStatusMetric => metrics_builder
                    .disputes_status_rate
                    .add_metrics_bucket(&value),
                DisputeMetrics::TotalAmountDisputed
                | DisputeMetrics::SessionizedTotalAmountDisputed => {
                    metrics_builder.disputed_amount.add_metrics_bucket(&value)
                }
                DisputeMetrics::TotalDisputeLostAmount
                | DisputeMetrics::SessionizedTotalDisputeLostAmount => metrics_builder
                    .dispute_lost_amount
                    .add_metrics_bucket(&value),
            }
        }

        logger::debug!(
            "Analytics Accumulated Results: metric: {}, results: {:#?}",
            metric,
            metrics_accumulator
        );
    }
    let mut total_disputed_amount = 0;
    let mut total_dispute_lost_amount = 0;
    let query_data: Vec<DisputeMetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let collected_values = val.collect();
            if let Some(amount) = collected_values.disputed_amount {
                total_disputed_amount += amount;
            }
            if let Some(amount) = collected_values.dispute_lost_amount {
                total_dispute_lost_amount += amount;
            }

            DisputeMetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();

    Ok(DisputesMetricsResponse {
        query_data,
        meta_data: [DisputesAnalyticsMetadata {
            total_disputed_amount: Some(total_disputed_amount),
            total_dispute_lost_amount: Some(total_dispute_lost_amount),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetDisputeFilterRequest,
    auth: &AuthInfo,
) -> AnalyticsResult<DisputeFiltersResponse> {
    let mut res = DisputeFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                            get_dispute_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                            get_dispute_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) => {
                let ckh_result = get_dispute_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_dispute_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
                    AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_dispute_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_dispute_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: DisputeFilterRow| match dim {
            DisputeDimensions::DisputeStage => fil.dispute_stage,
            DisputeDimensions::Connector => fil.connector,
            DisputeDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
        })
        .collect::<Vec<String>>();
        res.query_data.push(DisputeFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
