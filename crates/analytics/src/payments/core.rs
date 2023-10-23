#![allow(dead_code)]
use std::collections::HashMap;

use api_models::analytics::{
    payments::{
        MetricsBucketResponse, PaymentDimensions, PaymentMetrics, PaymentMetricsBucketIdentifier,
    },
    AnalyticsMetadata, FilterValue, GeneratePaymentReportRequest, GetPaymentFiltersRequest,
    GetPaymentMetricRequest, MetricsResponse, PaymentFiltersResponse, PaymentReportRequest,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_payment_filter_for_dimension, FilterRow},
    PaymentMetricsAccumulator,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    lambda_utils::invoke_lambda,
    metrics,
    payments::PaymentMetricAccumulator,
    AnalyticsProvider, PaymentReportConfig,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &str,
    req: GetPaymentMetricRequest,
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        PaymentMetricsBucketIdentifier,
        PaymentMetricsAccumulator,
    > = HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payments_query",
            payment_metric = metric_type.as_ref()
        );

        // TODO: lifetime issues with joinset,
        // can be optimized away if joinset lifetime requirements are relaxed
        let merchant_id_scoped = merchant_id.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_metrics(
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
        .into_report()
        .change_context(AnalyticsError::UnknownError)?
    {
        let data = data?;
        let attributes = &[
            metrics::request::add_attributes("metric_type", metric.to_string()),
            metrics::request::add_attributes(
                "source",
                match pool {
                    crate::AnalyticsProvider::Clickhouse(_) => "Clickhouse",
                    crate::AnalyticsProvider::Sqlx(_) => "Sqlx",
                    crate::AnalyticsProvider::CombinedCkh(_, _) => "CombinedCkh",
                    crate::AnalyticsProvider::CombinedSqlx(_, _) => "CombinedSqlx",
                },
            ),
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
}

#[instrument(skip_all)]
pub async fn generate_report(
    report_download_config: PaymentReportConfig,
    merchant_id: &str,
    user_email: &str,
    req: PaymentReportRequest,
) -> AnalyticsResult<()> {
    let lambda_req = GeneratePaymentReportRequest {
        request: req.clone(),
        merchant_id: merchant_id.to_string(),
        email: user_email.to_string(),
    };

    let json_bytes = serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
    invoke_lambda(report_download_config, &json_bytes).await?;

    Ok(())
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetPaymentFiltersRequest,
    merchant_id: &String,
) -> AnalyticsResult<PaymentFiltersResponse> {
    let mut res = PaymentFiltersResponse::default();

    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                get_payment_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_payment_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_poll,
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
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_poll,
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
        })
        .collect::<Vec<String>>();
        res.query_data.push(FilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
