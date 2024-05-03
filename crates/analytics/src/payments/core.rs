#![allow(dead_code)]
use std::collections::HashMap;

use api_models::analytics::{
    payments::{
        MetricsBucketResponse, PaymentDimensions, PaymentDistributions, PaymentMetrics,
        PaymentMetricsBucketIdentifier,
    },
    AnalyticsMetadata, FilterValue, GetPaymentFiltersRequest, GetPaymentMetricRequest,
    MetricsResponse, PaymentFiltersResponse,
};
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::{
    distribution::PaymentDistributionRow,
    filters::{get_payment_filter_for_dimension, FilterRow},
    metrics::PaymentMetricRow,
    PaymentMetricsAccumulator,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    metrics,
    payments::{PaymentDistributionAccumulator, PaymentMetricAccumulator},
    AnalyticsProvider,
};

#[derive(Debug)]
pub enum TaskType {
    MetricTask(
        PaymentMetrics,
        CustomResult<Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>, AnalyticsError>,
    ),
    DistributionTask(
        PaymentDistributions,
        CustomResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>, AnalyticsError>,
    ),
}

fn compare_and_return_matching(org_merchant_ids: &[String], payload: &[String]) -> Vec<String> {
    let matching_values: Vec<String> = payload
        .iter()
        .filter(|i| org_merchant_ids.contains(i))
        .cloned()
        .collect();

    if matching_values.is_empty() {
        org_merchant_ids.to_vec()
    } else {
        matching_values
    }
}

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,

    req: GetPaymentMetricRequest,
    merchant_ids: &[String],
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let org_merchant_ids = compare_and_return_matching(merchant_ids, &req.filters.merchant_id);
    let mut metrics_accumulator: HashMap<
        PaymentMetricsBucketIdentifier,
        PaymentMetricsAccumulator,
    > = HashMap::new();
    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payments_metrics_query",
            payment_metric = metric_type.as_ref()
        );

        // TODO: lifetime issues with joinset,
        // can be optimized away if joinset lifetime requirements are relaxed

        let merchant_ids = org_merchant_ids.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &req.filters,
                        &req.time_series.map(|t| t.granularity),
                        &req.time_range,
                        &merchant_ids.clone(),
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                TaskType::MetricTask(metric_type, data)
            }
            .instrument(task_span),
        );
    }

    if let Some(distribution) = req.clone().distribution {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payments_distribution_query",
            payment_distribution = distribution.distribution_for.as_ref()
        );

        let merchant_ids = org_merchant_ids.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_distribution(
                        &distribution,
                        &req.group_by_names.clone(),
                        &merchant_ids,
                        &req.filters,
                        &req.time_series.map(|t| t.granularity),
                        &req.time_range,
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                TaskType::DistributionTask(distribution.distribution_for, data)
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
                        PaymentMetrics::PaymentSuccessRate => metrics_builder
                            .payment_success_rate
                            .add_metrics_bucket(&value),
                        PaymentMetrics::PaymentCount => {
                            metrics_builder.payment_count.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::PaymentSuccessCount => {
                            metrics_builder.payment_success.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::PaymentProcessedAmount => {
                            metrics_builder.processed_amount.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::AvgTicketSize => {
                            metrics_builder.avg_ticket_size.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::RetriesCount => {
                            metrics_builder.retries_count.add_metrics_bucket(&value);
                            metrics_builder
                                .retries_amount_processed
                                .add_metrics_bucket(&value)
                        }
                        PaymentMetrics::ConnectorSuccessRate => {
                            metrics_builder
                                .connector_success_rate
                                .add_metrics_bucket(&value);
                        }
                    }
                }

                logger::debug!(
                    "Analytics Accumulated Results: metric: {}, results: {:#?}",
                    metric,
                    metrics_accumulator
                );
            }
            TaskType::DistributionTask(distribution, data) => {
                let data = data?;
                let attributes = &[
                    metrics::request::add_attributes("distribution_type", distribution.to_string()),
                    metrics::request::add_attributes("source", pool.to_string()),
                ];

                let value = u64::try_from(data.len());
                if let Ok(val) = value {
                    metrics::BUCKETS_FETCHED.record(&metrics::CONTEXT, val, attributes);
                    logger::debug!("Attributes: {:?}, Buckets fetched: {}", attributes, val);
                }

                for (id, value) in data {
                    logger::debug!(bucket_id=?id, bucket_value=?value, "Bucket row for distribution {distribution}");
                    let metrics_accumulator = metrics_accumulator.entry(id).or_default();
                    match distribution {
                        PaymentDistributions::PaymentErrorMessage => metrics_accumulator
                            .payment_error_message
                            .add_distribution_bucket(&value),
                    }
                }

                logger::debug!(
                    "Analytics Accumulated Results: distribution: {}, results: {:#?}",
                    distribution,
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
    req: GetPaymentFiltersRequest,
    merchant_ids: &[String],
) -> AnalyticsResult<PaymentFiltersResponse> {
    let mut res = PaymentFiltersResponse::default();

    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                get_payment_filter_for_dimension(dim,  &req.time_range, pool,merchant_ids)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_payment_filter_for_dimension(dim,  &req.time_range, pool,merchant_ids)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    &req.time_range,
                    ckh_pool,
                    merchant_ids
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    &req.time_range,
                    sqlx_poll,
                    merchant_ids
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
                    AnalyticsProvider::CombinedSqlx(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    &req.time_range,
                    ckh_pool,
                    merchant_ids
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    &req.time_range,
                    sqlx_poll,
                    merchant_ids
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: FilterRow| match dim {
            PaymentDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentStatus => fil.status.map(|i| i.as_ref().to_string()),
            PaymentDimensions::Connector => fil.connector,
            PaymentDimensions::AuthType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentMethod => fil.payment_method,
            PaymentDimensions::PaymentMethodType => fil.payment_method_type,
            PaymentDimensions::MerchantId=>fil.merchant_id
        })
        .collect::<Vec<String>>();
        res.query_data.push(FilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
