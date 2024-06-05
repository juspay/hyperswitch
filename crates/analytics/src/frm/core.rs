#![allow(dead_code)]
use std::collections::HashMap;

use api_models::analytics::{
    frm::{FrmDimensions, FrmMetrics, FrmMetricsBucketIdentifier, FrmMetricsBucketResponse},
    AnalyticsMetadata, FrmFilterValue, FrmFiltersResponse, GetFrmFilterRequest,
    GetFrmMetricRequest, MetricsResponse,
};
use error_stack::ResultExt;
use router_env::{
    logger,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_frm_filter_for_dimension, FrmFilterRow},
    FrmMetricsAccumulator,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    frm::FrmMetricAccumulator,
    metrics,
    types::FiltersError,
    AnalyticsProvider,
};

pub async fn get_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &String,
    req: GetFrmMetricRequest,
) -> AnalyticsResult<MetricsResponse<FrmMetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<FrmMetricsBucketIdentifier, FrmMetricsAccumulator> =
        HashMap::new();
    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span =
            tracing::debug_span!("analytics_frm_query", frm_metric = metric_type.as_ref());
        // Currently JoinSet works with only static lifetime references even if the task pool does not outlive the given reference
        // We can optimize away this clone once that is fixed
        let merchant_id_scoped = merchant_id.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_frm_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &merchant_id_scoped,
                        &req.filters,
                        &req.time_series.map(|t| t.granularity),
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
        let attributes = &[
            metrics::request::add_attributes("metric_type", metric.to_string()),
            metrics::request::add_attributes("source", pool.to_string()),
        ];

        let value = u64::try_from(data.len());
        if let Ok(val) = value {
            metrics::BUCKETS_FETCHED.record(&metrics::CONTEXT, val, attributes);
            logger::debug!("Attributes: {:?}, Buckets fetched: {}", attributes, val);
        }

        for (id, value) in data {
            logger::debug!(bucket_id=?id, bucket_value=?value, "Bucket row for metric {metric}");
            let metrics_builder = metrics_accumulator.entry(id).or_default();
            match metric {
                FrmMetrics::FrmBlockedRate => {
                    metrics_builder.frm_blocked_rate.add_metrics_bucket(&value)
                }
                FrmMetrics::FrmTriggeredAttempts => metrics_builder
                    .frm_triggered_attempts
                    .add_metrics_bucket(&value),
            }
        }

        logger::debug!(
            "Analytics Accumulated Results: metric: {}, results: {:#?}",
            metric,
            metrics_accumulator
        );
    }
    let query_data: Vec<FrmMetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| FrmMetricsBucketResponse {
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
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetFrmFilterRequest,
    merchant_id: &String,
) -> AnalyticsResult<FrmFiltersResponse> {
    let mut res = FrmFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
            AnalyticsProvider::Clickhouse(_pool) => Err(FiltersError::NotImplemented(
                "Frm not implemented for Clickhouse",
            ))
            .attach_printable("Clickhouse Analytics is not implemented for Clickhouse"),
            AnalyticsProvider::Sqlx(sqlx_pool)
            | AnalyticsProvider::CombinedCkh(sqlx_pool, _)
            | AnalyticsProvider::CombinedSqlx(sqlx_pool, _) => {
                get_frm_filter_for_dimension(dim, merchant_id, &req.time_range, sqlx_pool).await
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: FrmFilterRow| match dim {
            FrmDimensions::FrmStatus => fil.frm_status.map(|i| i.as_ref().to_string()),
            FrmDimensions::FrmName => fil.frm_name,
            FrmDimensions::FrmTransactionType => {
                fil.frm_transaction_type.map(|i| i.as_ref().to_string())
            }
        })
        .collect::<Vec<String>>();
        res.query_data.push(FrmFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
