use std::collections::HashMap;

use api_models::analytics::{
    refunds::{RefundMetrics, RefundMetricsBucketIdentifier, RefundMetricsBucketResponse},
    AnalyticsMetadata, GetRefundMetricRequest, MetricsResponse,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{
    logger,
    tracing::{self, Instrument},
};

use super::RefundMetricsAccumulator;
use crate::{
    analytics::{
        core::AnalyticsApiResponse, errors::AnalyticsError, refunds::RefundMetricAccumulator,
        AnalyticsProvider,
    },
    services::ApplicationResponse,
    types::domain,
};

pub async fn get_metrics(
    pool: AnalyticsProvider,
    merchant_account: domain::MerchantAccount,
    req: GetRefundMetricRequest,
) -> AnalyticsApiResponse<MetricsResponse<RefundMetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<RefundMetricsBucketIdentifier, RefundMetricsAccumulator> =
        HashMap::new();
    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let merchant_id = merchant_account.merchant_id.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_refund_query",
            refund_metric = metric_type.as_ref()
        );
        set.spawn(
            async move {
                let data = pool
                    .get_refund_metrics(
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
        for (id, value) in data? {
            logger::debug!(bucket_id=?id, bucket_value=?value, "Bucket row for metric {metric}");
            let metrics_builder = metrics_accumulator.entry(id).or_default();
            match metric {
                RefundMetrics::RefundSuccessRate => metrics_builder
                    .refund_success_rate
                    .add_metrics_bucket(&value),
                RefundMetrics::RefundCount => {
                    metrics_builder.refund_count.add_metrics_bucket(&value)
                }
                RefundMetrics::RefundSuccessCount => {
                    metrics_builder.refund_success.add_metrics_bucket(&value)
                }
                RefundMetrics::RefundProcessedAmount => {
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
    let query_data: Vec<RefundMetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| RefundMetricsBucketResponse {
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
