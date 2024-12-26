#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use api_models::analytics::{
    payment_intents::{
        MetricsBucketResponse, PaymentIntentDimensions, PaymentIntentMetrics,
        PaymentIntentMetricsBucketIdentifier,
    },
    GetPaymentIntentFiltersRequest, GetPaymentIntentMetricRequest, PaymentIntentFilterValue,
    PaymentIntentFiltersResponse, PaymentIntentsAnalyticsMetadata, PaymentIntentsMetricsResponse,
};
use bigdecimal::ToPrimitive;
use common_enums::Currency;
use common_utils::{errors::CustomResult, types::TimeRange};
use currency_conversion::{conversion::convert, types::ExchangeRates};
use error_stack::ResultExt;
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::{
    filters::{get_payment_intent_filter_for_dimension, PaymentIntentFilterRow},
    metrics::PaymentIntentMetricRow,
    sankey::{get_sankey_data, SankeyRow},
    PaymentIntentMetricsAccumulator,
};
use crate::{
    enums::AuthInfo,
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
            HashSet<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>,
            AnalyticsError,
        >,
    ),
}

#[instrument(skip_all)]
pub async fn get_sankey(
    pool: &AnalyticsProvider,
    auth: &AuthInfo,
    req: TimeRange,
) -> AnalyticsResult<Vec<SankeyRow>> {
    match pool {
        AnalyticsProvider::Sqlx(_) => Err(AnalyticsError::NotImplemented(
            "Sankey not implemented for sqlx",
        ))?,
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool) => {
            let sankey_rows = get_sankey_data(ckh_pool, auth, &req)
                .await
                .change_context(AnalyticsError::UnknownError)?;
            Ok(sankey_rows)
        }
    }
}

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    ex_rates: &Option<ExchangeRates>,
    auth: &AuthInfo,
    req: GetPaymentIntentMetricRequest,
) -> AnalyticsResult<PaymentIntentsMetricsResponse<MetricsBucketResponse>> {
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
        let auth_scoped = auth.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_intent_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &auth_scoped,
                        &req.filters,
                        req.time_series.map(|t| t.granularity),
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
                        PaymentIntentMetrics::SuccessfulSmartRetries
                        | PaymentIntentMetrics::SessionizedSuccessfulSmartRetries => {
                            metrics_builder
                                .successful_smart_retries
                                .add_metrics_bucket(&value)
                        }
                        PaymentIntentMetrics::TotalSmartRetries
                        | PaymentIntentMetrics::SessionizedTotalSmartRetries => metrics_builder
                            .total_smart_retries
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::SmartRetriedAmount
                        | PaymentIntentMetrics::SessionizedSmartRetriedAmount => metrics_builder
                            .smart_retried_amount
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::PaymentIntentCount
                        | PaymentIntentMetrics::SessionizedPaymentIntentCount => metrics_builder
                            .payment_intent_count
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::PaymentsSuccessRate
                        | PaymentIntentMetrics::SessionizedPaymentsSuccessRate => metrics_builder
                            .payments_success_rate
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::SessionizedPaymentProcessedAmount
                        | PaymentIntentMetrics::PaymentProcessedAmount => metrics_builder
                            .payment_processed_amount
                            .add_metrics_bucket(&value),
                        PaymentIntentMetrics::SessionizedPaymentsDistribution => metrics_builder
                            .payments_distribution
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

    let mut success = 0;
    let mut success_without_smart_retries = 0;
    let mut total_smart_retried_amount = 0;
    let mut total_smart_retried_amount_in_usd = 0;
    let mut total_smart_retried_amount_without_smart_retries = 0;
    let mut total_smart_retried_amount_without_smart_retries_in_usd = 0;
    let mut total = 0;
    let mut total_payment_processed_amount = 0;
    let mut total_payment_processed_amount_in_usd = 0;
    let mut total_payment_processed_count = 0;
    let mut total_payment_processed_amount_without_smart_retries = 0;
    let mut total_payment_processed_amount_without_smart_retries_in_usd = 0;
    let mut total_payment_processed_count_without_smart_retries = 0;
    let query_data: Vec<MetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let mut collected_values = val.collect();
            if let Some(success_count) = collected_values.successful_payments {
                success += success_count;
            }
            if let Some(success_count) = collected_values.successful_payments_without_smart_retries
            {
                success_without_smart_retries += success_count;
            }
            if let Some(total_count) = collected_values.total_payments {
                total += total_count;
            }
            if let Some(retried_amount) = collected_values.smart_retried_amount {
                let amount_in_usd = if let Some(ex_rates) = ex_rates {
                    id.currency
                        .and_then(|currency| {
                            i64::try_from(retried_amount)
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
                        .unwrap_or_default()
                } else {
                    None
                };
                collected_values.smart_retried_amount_in_usd = amount_in_usd;
                total_smart_retried_amount += retried_amount;
                total_smart_retried_amount_in_usd += amount_in_usd.unwrap_or(0);
            }
            if let Some(retried_amount) =
                collected_values.smart_retried_amount_without_smart_retries
            {
                let amount_in_usd = if let Some(ex_rates) = ex_rates {
                    id.currency
                        .and_then(|currency| {
                            i64::try_from(retried_amount)
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
                        .unwrap_or_default()
                } else {
                    None
                };
                collected_values.smart_retried_amount_without_smart_retries_in_usd = amount_in_usd;
                total_smart_retried_amount_without_smart_retries += retried_amount;
                total_smart_retried_amount_without_smart_retries_in_usd +=
                    amount_in_usd.unwrap_or(0);
            }
            if let Some(amount) = collected_values.payment_processed_amount {
                let amount_in_usd = if let Some(ex_rates) = ex_rates {
                    id.currency
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
                        .unwrap_or_default()
                } else {
                    None
                };
                collected_values.payment_processed_amount_in_usd = amount_in_usd;
                total_payment_processed_amount_in_usd += amount_in_usd.unwrap_or(0);
                total_payment_processed_amount += amount;
            }
            if let Some(count) = collected_values.payment_processed_count {
                total_payment_processed_count += count;
            }
            if let Some(amount) = collected_values.payment_processed_amount_without_smart_retries {
                let amount_in_usd = if let Some(ex_rates) = ex_rates {
                    id.currency
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
                        .unwrap_or_default()
                } else {
                    None
                };
                collected_values.payment_processed_amount_without_smart_retries_in_usd =
                    amount_in_usd;
                total_payment_processed_amount_without_smart_retries_in_usd +=
                    amount_in_usd.unwrap_or(0);
                total_payment_processed_amount_without_smart_retries += amount;
            }
            if let Some(count) = collected_values.payment_processed_count_without_smart_retries {
                total_payment_processed_count_without_smart_retries += count;
            }
            MetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();
    let total_success_rate = match (success, total) {
        (s, t) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
        _ => None,
    };
    let total_success_rate_without_smart_retries = match (success_without_smart_retries, total) {
        (s, t) if t > 0 => Some(f64::from(s) * 100.0 / f64::from(t)),
        _ => None,
    };
    Ok(PaymentIntentsMetricsResponse {
        query_data,
        meta_data: [PaymentIntentsAnalyticsMetadata {
            total_success_rate,
            total_success_rate_without_smart_retries,
            total_smart_retried_amount: Some(total_smart_retried_amount),
            total_smart_retried_amount_without_smart_retries: Some(
                total_smart_retried_amount_without_smart_retries,
            ),
            total_payment_processed_amount: Some(total_payment_processed_amount),
            total_payment_processed_amount_without_smart_retries: Some(
                total_payment_processed_amount_without_smart_retries,
            ),
            total_smart_retried_amount_in_usd: if ex_rates.is_some() {
                Some(total_smart_retried_amount_in_usd)
            } else {
                None
            },
            total_smart_retried_amount_without_smart_retries_in_usd: if ex_rates.is_some() {
                Some(total_smart_retried_amount_without_smart_retries_in_usd)
            } else {
                None
            },
            total_payment_processed_amount_in_usd: if ex_rates.is_some() {
                Some(total_payment_processed_amount_in_usd)
            } else {
                None
            },
            total_payment_processed_amount_without_smart_retries_in_usd: if ex_rates.is_some() {
                Some(total_payment_processed_amount_without_smart_retries_in_usd)
            } else {
                None
            },
            total_payment_processed_count: Some(total_payment_processed_count),
            total_payment_processed_count_without_smart_retries: Some(
                total_payment_processed_count_without_smart_retries,
            ),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetPaymentIntentFiltersRequest,
    merchant_id: &common_utils::id_type::MerchantId,
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
            PaymentIntentDimensions::ProfileId => fil.profile_id,
            PaymentIntentDimensions::Connector => fil.connector,
            PaymentIntentDimensions::AuthType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            PaymentIntentDimensions::PaymentMethod => fil.payment_method,
            PaymentIntentDimensions::PaymentMethodType => fil.payment_method_type,
            PaymentIntentDimensions::CardNetwork => fil.card_network,
            PaymentIntentDimensions::MerchantId => fil.merchant_id,
            PaymentIntentDimensions::CardLast4 => fil.card_last_4,
            PaymentIntentDimensions::CardIssuer => fil.card_issuer,
            PaymentIntentDimensions::ErrorReason => fil.error_reason,
        })
        .collect::<Vec<String>>();
        res.query_data.push(PaymentIntentFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
