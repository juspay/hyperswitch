use std::collections::HashMap;

use api_models::analytics::{
    payments::{MetricsBucketResponse, PaymentMetrics, PaymentMetricsBucketIdentifier},
    AnalyticsMetadata, GetPaymentMetricRequest, MetricsResponse,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::PaymentMetricsAccumulator;
use crate::{
    analytics::{
        core::AnalyticsApiResponse, errors::AnalyticsError, metrics,
        payments::PaymentMetricAccumulator, AnalyticsProvider,
    },
    services::ApplicationResponse,
    types::domain,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: AnalyticsProvider,
    merchant_account: domain::MerchantAccount,
    req: GetPaymentMetricRequest,
) -> AnalyticsApiResponse<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        PaymentMetricsBucketIdentifier,
        PaymentMetricsAccumulator,
    > = HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let merchant_id = merchant_account.merchant_id.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_payments_query",
            payment_metric = metric_type.as_ref()
        );
        set.spawn(
            async move {
                let data = pool
                    .get_payment_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &merchant_id,
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
                    crate::analytics::AnalyticsProvider::Sqlx(_) => "Sqlx",
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

    Ok(ApplicationResponse::Json(MetricsResponse {
        query_data,
        meta_data: [AnalyticsMetadata {
            current_time_range: req.time_range,
        }],
    }))
}
