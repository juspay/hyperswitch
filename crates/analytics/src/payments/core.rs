#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use api_models::analytics::{
    payments::{
        MetricsBucketResponse, PaymentDimensions, PaymentDistributions, PaymentMetrics,
        PaymentMetricsBucketIdentifier,
    },
    FilterValue, GetPaymentFiltersRequest, GetPaymentMetricRequest, PaymentFiltersResponse,
    PaymentsAnalyticsMetadata, PaymentsMetricsResponse,
};
use bigdecimal::ToPrimitive;
use common_enums::Currency;
use common_utils::errors::CustomResult;
use currency_conversion::{conversion::convert, types::ExchangeRates};
use error_stack::ResultExt;
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::{
    distribution::PaymentDistributionRow,
    filters::{get_payment_filter_for_dimension, PaymentFilterRow},
    metrics::PaymentMetricRow,
    PaymentMetricsAccumulator,
};
use crate::{
    enums::AuthInfo,
    errors::{AnalyticsError, AnalyticsResult},
    metrics,
    payments::{PaymentDistributionAccumulator, PaymentMetricAccumulator},
    AnalyticsProvider,
};

#[derive(Debug)]
pub enum TaskType {
    MetricTask(
        PaymentMetrics,
        CustomResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>, AnalyticsError>,
    ),
    DistributionTask(
        PaymentDistributions,
        CustomResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>, AnalyticsError>,
    ),
}

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    ex_rates: &Option<ExchangeRates>,
    auth: &AuthInfo,
    req: GetPaymentMetricRequest,
) -> AnalyticsResult<PaymentsMetricsResponse<MetricsBucketResponse>> {
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
        let auth_scoped = auth.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_metrics(
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

    if let Some(distribution) = req.clone().distribution {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payments_distribution_query",
            payment_distribution = distribution.distribution_for.as_ref()
        );

        let auth_scoped = auth.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_payment_distribution(
                        &distribution,
                        &req.group_by_names.clone(),
                        &auth_scoped,
                        &req.filters,
                        req.time_series.map(|t| t.granularity),
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
                        PaymentMetrics::PaymentSuccessRate
                        | PaymentMetrics::SessionizedPaymentSuccessRate => metrics_builder
                            .payment_success_rate
                            .add_metrics_bucket(&value),
                        PaymentMetrics::PaymentCount | PaymentMetrics::SessionizedPaymentCount => {
                            metrics_builder.payment_count.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::PaymentSuccessCount
                        | PaymentMetrics::SessionizedPaymentSuccessCount => {
                            metrics_builder.payment_success.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::PaymentProcessedAmount
                        | PaymentMetrics::SessionizedPaymentProcessedAmount => {
                            metrics_builder.processed_amount.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::AvgTicketSize
                        | PaymentMetrics::SessionizedAvgTicketSize => {
                            metrics_builder.avg_ticket_size.add_metrics_bucket(&value)
                        }
                        PaymentMetrics::RetriesCount | PaymentMetrics::SessionizedRetriesCount => {
                            metrics_builder.retries_count.add_metrics_bucket(&value);
                            metrics_builder
                                .retries_amount_processed
                                .add_metrics_bucket(&value)
                        }
                        PaymentMetrics::ConnectorSuccessRate
                        | PaymentMetrics::SessionizedConnectorSuccessRate => {
                            metrics_builder
                                .connector_success_rate
                                .add_metrics_bucket(&value);
                        }
                        PaymentMetrics::DebitRouting | PaymentMetrics::SessionizedDebitRouting => {
                            metrics_builder.debit_routing.add_metrics_bucket(&value);
                        }
                        PaymentMetrics::PaymentsDistribution => {
                            metrics_builder
                                .payments_distribution
                                .add_metrics_bucket(&value);
                        }
                        PaymentMetrics::FailureReasons => {
                            metrics_builder
                                .failure_reasons_distribution
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
                let attributes = router_env::metric_attributes!(
                    ("distribution_type", distribution.to_string()),
                    ("source", pool.to_string()),
                );

                let value = u64::try_from(data.len());
                if let Ok(val) = value {
                    metrics::BUCKETS_FETCHED.record(val, attributes);
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
    let mut total_payment_processed_amount = 0;
    let mut total_payment_processed_count = 0;
    let mut total_payment_processed_amount_without_smart_retries = 0;
    let mut total_payment_processed_count_without_smart_retries = 0;
    let mut total_failure_reasons_count = 0;
    let mut total_failure_reasons_count_without_smart_retries = 0;
    let mut total_payment_processed_amount_in_usd = 0;
    let mut total_payment_processed_amount_without_smart_retries_usd = 0;
    let query_data: Vec<MetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let mut collected_values = val.collect();
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
                total_payment_processed_amount += amount;
                total_payment_processed_amount_in_usd += amount_in_usd.unwrap_or(0);
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
                collected_values.payment_processed_amount_without_smart_retries_usd = amount_in_usd;
                total_payment_processed_amount_without_smart_retries += amount;
                total_payment_processed_amount_without_smart_retries_usd +=
                    amount_in_usd.unwrap_or(0);
            }
            if let Some(count) = collected_values.payment_processed_count_without_smart_retries {
                total_payment_processed_count_without_smart_retries += count;
            }
            if let Some(count) = collected_values.failure_reason_count {
                total_failure_reasons_count += count;
            }
            if let Some(count) = collected_values.failure_reason_count_without_smart_retries {
                total_failure_reasons_count_without_smart_retries += count;
            }
            if let Some(savings) = collected_values.debit_routing_savings {
                let savings_in_usd = if let Some(ex_rates) = ex_rates {
                    id.currency
                        .and_then(|currency| {
                            i64::try_from(savings)
                                .inspect_err(|e| {
                                    logger::error!(
                                        "Debit Routing savings conversion error: {:?}",
                                        e
                                    )
                                })
                                .ok()
                                .and_then(|savings_i64| {
                                    convert(ex_rates, currency, Currency::USD, savings_i64)
                                        .inspect_err(|e| {
                                            logger::error!("Currency conversion error: {:?}", e)
                                        })
                                        .ok()
                                })
                        })
                        .map(|savings| (savings * rust_decimal::Decimal::new(100, 0)).to_u64())
                        .unwrap_or_default()
                } else {
                    None
                };
                collected_values.debit_routing_savings_in_usd = savings_in_usd;
            }
            MetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();
    Ok(PaymentsMetricsResponse {
        query_data,
        meta_data: [PaymentsAnalyticsMetadata {
            total_payment_processed_amount: Some(total_payment_processed_amount),
            total_payment_processed_amount_in_usd: if ex_rates.is_some() {
                Some(total_payment_processed_amount_in_usd)
            } else {
                None
            },
            total_payment_processed_amount_without_smart_retries: Some(
                total_payment_processed_amount_without_smart_retries,
            ),
            total_payment_processed_amount_without_smart_retries_usd: if ex_rates.is_some() {
                Some(total_payment_processed_amount_without_smart_retries_usd)
            } else {
                None
            },
            total_payment_processed_count: Some(total_payment_processed_count),
            total_payment_processed_count_without_smart_retries: Some(
                total_payment_processed_count_without_smart_retries,
            ),
            total_failure_reasons_count: Some(total_failure_reasons_count),
            total_failure_reasons_count_without_smart_retries: Some(
                total_failure_reasons_count_without_smart_retries,
            ),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetPaymentFiltersRequest,
    auth: &AuthInfo,
) -> AnalyticsResult<PaymentFiltersResponse> {
    let mut res = PaymentFiltersResponse::default();

    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                get_payment_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_payment_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    auth,
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
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    auth,
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
        .filter_map(|fil: PaymentFilterRow| match dim {
            PaymentDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentStatus => fil.status.map(|i| i.as_ref().to_string()),
            PaymentDimensions::Connector => fil.connector,
            PaymentDimensions::AuthType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentMethod => fil.payment_method,
            PaymentDimensions::PaymentMethodType => fil.payment_method_type,
            PaymentDimensions::ClientSource => fil.client_source,
            PaymentDimensions::ClientVersion => fil.client_version,
            PaymentDimensions::ProfileId => fil.profile_id,
            PaymentDimensions::CardNetwork => fil.card_network,
            PaymentDimensions::MerchantId => fil.merchant_id,
            PaymentDimensions::CardLast4 => fil.card_last_4,
            PaymentDimensions::CardIssuer => fil.card_issuer,
            PaymentDimensions::ErrorReason => fil.error_reason,
            PaymentDimensions::RoutingApproach => fil.routing_approach.map(|i| i.as_ref().to_string()),
            PaymentDimensions::SignatureNetwork => fil.signature_network,
            PaymentDimensions::IsIssuerRegulated => fil.is_issuer_regulated.map(|b| b.to_string()),
            PaymentDimensions::IsDebitRouted => fil.is_debit_routed.map(|b| b.to_string())
        })
        .collect::<Vec<String>>();
        res.query_data.push(FilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
