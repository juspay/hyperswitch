#![allow(dead_code)]
use std::collections::HashMap;

use api_models::analytics::{
    payment_intents::{
        MetricsBucketResponse, PaymentIntentDimensions, PaymentIntentMetrics,
        PaymentIntentMetricsBucketIdentifier,
    },
    AnalyticsMetadata, GetPaymentIntentFiltersRequest, GetPaymentIntentMetricRequest,
    MetricsResponse, PaymentIntentFilterValue, PaymentIntentFiltersResponse,
};
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use router_env::{
    instrument, logger,
    metrics::add_attributes,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_payment_intent_filter_for_dimension, PaymentIntentFilterRow},
    metrics::PaymentIntentMetricRow,
    PaymentIntentMetricsAccumulator,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    metrics,
    payment_intents::PaymentIntentMetricAccumulator,
    AnalyticsProvider,
};

#[derive(Debug)]
pub enum TaskType {
    MetricTask(
        PaymentIntentMetrics,
        CustomResult<
            Vec<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>,
            AnalyticsError,
        >,
    ),
}

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &str,
    req: GetPaymentIntentMetricRequest,
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        PaymentIntentMetricsBucketIdentifier,
        PaymentIntentMetricsAccumulator,
    > = HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payment_intents_metrics_query",
            payment_metric = metric_type.as_ref()
        );

        // TODO: lifetime issues with joinset,
        // can be optimized away if joinset lifetime requirements are relaxed
        let merchant_id_scoped = merchant_id.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_intent_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &merchant_id_scoped,
                        &req.filters,
                        &req.time_series.map(|t| t.granularity),
                        &req.time_range,
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                TaskType::MetricTask(metric_type, data)
            }
            .instrument(task_span),
        );
    }

    while let Some(task_type) = set
        .join_next()
        .await
        .transpose()
        .change_context(AnalyticsError::UnknownError)?
    {
        match task_type {
            TaskType::MetricTask(metric, data) => {
                let data = data?;
                let attributes = &add_attributes([
                    ("metric_type", metric.to_string()),
                    ("source", pool.to_string()),
                ]);

                let value = u64::try_from(data.len());
                if let Ok(val) = value {
                    metrics::BUCKETS_FETCHED.record(&metrics::CONTEXT, val, attributes);
                    logger::debug!("Attributes: {:?}, Buckets fetched: {}", attributes, val);
                }

                for (id, value) in data {
                    logger::debug!(bucket_id=?id, bucket_value=?value, "Bucket row for metric {metric}");
                    let metrics_builder = metrics_accumulator.entry(id).or_default();
                    match metric {
                        PaymentIntentMetrics::SuccessfulSmartRetries => metrics_builder
                            .successful_smart_retries
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::TotalSmartRetries => metrics_builder
                            .total_smart_retries
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::SmartRetriedAmount => metrics_builder
                            .smart_retried_amount
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::PaymentIntentCount => metrics_builder
                            .payment_intent_count
                            .add_metrics_bucket(&value),
                    }
                }

                logger::debug!(
                    "Analytics Accumulated Results: metric: {}, results: {:#?}",
                    metric,
                    metrics_accumulator
                );
            }
        }
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
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetPaymentIntentFiltersRequest,
    merchant_id: &String,
) -> AnalyticsResult<PaymentIntentFiltersResponse> {
    let mut res = PaymentIntentFiltersResponse::default();

    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                get_payment_intent_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_payment_intent_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_intent_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_intent_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_poll,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payment intents analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
                    AnalyticsProvider::CombinedSqlx(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_intent_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_intent_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_poll,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payment intents analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: PaymentIntentFilterRow| match dim {
            PaymentIntentDimensions::PaymentIntentStatus => fil.status.map(|i| i.as_ref().to_string()),
            PaymentIntentDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
        })
        .collect::<Vec<String>>();
        res.query_data.push(PaymentIntentFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
