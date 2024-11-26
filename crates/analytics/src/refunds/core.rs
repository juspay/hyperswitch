#![allow(dead_code)]
use std::collections::HashMap;

use api_models::analytics::{
    refunds::{
        RefundDimensions, RefundMetrics, RefundMetricsBucketIdentifier, RefundMetricsBucketResponse,
    },
    GetRefundFilterRequest, GetRefundMetricRequest, RefundFilterValue, RefundFiltersResponse,
    RefundsAnalyticsMetadata, RefundsMetricsResponse,
};
use bigdecimal::ToPrimitive;
use common_enums::Currency;
use currency_conversion::{conversion::convert, types::ExchangeRates};
use error_stack::ResultExt;
use router_env::{
    logger,
    metrics::add_attributes,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_refund_filter_for_dimension, RefundFilterRow},
    RefundMetricsAccumulator,
};
use crate::{
    enums::AuthInfo,
    errors::{AnalyticsError, AnalyticsResult},
    metrics,
    refunds::RefundMetricAccumulator,
    AnalyticsProvider,
};

pub async fn get_metrics(
    pool: &AnalyticsProvider,
    ex_rates: &ExchangeRates,
    auth: &AuthInfo,
    req: GetRefundMetricRequest,
) -> AnalyticsResult<RefundsMetricsResponse<RefundMetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<RefundMetricsBucketIdentifier, RefundMetricsAccumulator> =
        HashMap::new();
    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_refund_query",
            refund_metric = metric_type.as_ref()
        );
        // Currently JoinSet works with only static lifetime references even if the task pool does not outlive the given reference
        // We can optimize away this clone once that is fixed
        let auth_scoped = auth.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_refund_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &auth_scoped,
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
                RefundMetrics::RefundSuccessRate | RefundMetrics::SessionizedRefundSuccessRate => {
                    metrics_builder
                        .refund_success_rate
                        .add_metrics_bucket(&value)
                }
                RefundMetrics::RefundCount | RefundMetrics::SessionizedRefundCount => {
                    metrics_builder.refund_count.add_metrics_bucket(&value)
                }
                RefundMetrics::RefundSuccessCount
                | RefundMetrics::SessionizedRefundSuccessCount => {
                    metrics_builder.refund_success.add_metrics_bucket(&value)
                }
                RefundMetrics::RefundProcessedAmount
                | RefundMetrics::SessionizedRefundProcessedAmount => {
                    metrics_builder.processed_amount.add_metrics_bucket(&value)
                }
            }
        }

        logger::debug!(
            "Analytics Accumulated Results: metric: {}, results: {:#?}",
            metric,
            metrics_accumulator
        );
    }
    let mut total_refund_processed_amount = 0;
    let mut total_refund_processed_amount_in_usd = 0;
    let query_data: Vec<RefundMetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let mut collected_values = val.collect();
            if let Some(amount) = collected_values.refund_processed_amount {
                let amount_in_usd = id
                    .currency
                    .and_then(|currency| {
                        i64::try_from(amount)
                            .inspect_err(|e| logger::error!("Amount conversion error: {:?}", e))
                            .ok()
                            .and_then(|amount_i64| {
                                convert(ex_rates, currency, Currency::USD, amount_i64)
                                    .inspect_err(|e| {
                                        logger::error!("Currency conversion error: {:?}", e)
                                    })
                                    .ok()
                            })
                    })
                    .map(|amount| (amount * rust_decimal::Decimal::new(100, 0)).to_u64())
                    .unwrap_or_default();
                collected_values.refund_processed_amount_in_usd = amount_in_usd;
                total_refund_processed_amount += amount;
                total_refund_processed_amount_in_usd += amount_in_usd.unwrap_or(0);
            }
            RefundMetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();

    Ok(RefundsMetricsResponse {
        query_data,
        meta_data: [RefundsAnalyticsMetadata {
            total_refund_processed_amount: Some(total_refund_processed_amount),
            total_refund_processed_amount_in_usd: Some(total_refund_processed_amount_in_usd),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetRefundFilterRequest,
    auth: &AuthInfo,
) -> AnalyticsResult<RefundFiltersResponse> {
    let mut res = RefundFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                get_refund_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_refund_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) => {
                let ckh_result = get_refund_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_refund_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
                    AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_refund_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_refund_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: RefundFilterRow| match dim {
            RefundDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            RefundDimensions::RefundStatus => fil.refund_status.map(|i| i.as_ref().to_string()),
            RefundDimensions::Connector => fil.connector,
            RefundDimensions::RefundType => fil.refund_type.map(|i| i.as_ref().to_string()),
            RefundDimensions::ProfileId => fil.profile_id,
        })
        .collect::<Vec<String>>();
        res.query_data.push(RefundFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
