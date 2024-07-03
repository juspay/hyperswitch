use std::collections::HashMap;

use api_models::analytics::{
    active_payments::{
        ActivePaymentsMetrics, ActivePaymentsMetricsBucketIdentifier, MetricsBucketResponse,
    },
    AnalyticsMetadata, GetActivePaymentsMetricRequest, MetricsResponse,
};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use super::ActivePaymentsMetricsAccumulator;
use crate::{
    active_payments::ActivePaymentsMetricAccumulator,
    errors::{AnalyticsError, AnalyticsResult},
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    publishable_key: Option<&String>,
    merchant_id: Option<&String>,
    req: GetActivePaymentsMetricRequest,
) -> AnalyticsResult<MetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        ActivePaymentsMetricsBucketIdentifier,
        ActivePaymentsMetricsAccumulator,
    > = HashMap::new();

    if let Some(publishable_key) = publishable_key {
        if let Some(merchant_id) = merchant_id {
            let mut set = tokio::task::JoinSet::new();
            for metric_type in req.metrics.iter().cloned() {
                let publishable_key_scoped = publishable_key.to_owned();
                let merchant_id_scoped = merchant_id.to_owned();
                let pool = pool.clone();
                set.spawn(async move {
                    let data = pool
                        .get_active_payments_metrics(
                            &metric_type,
                            &merchant_id_scoped,
                            &publishable_key_scoped,
                            &req.time_range,
                        )
                        .await
                        .change_context(AnalyticsError::UnknownError);
                    (metric_type, data)
                });
            }

            while let Some((metric, data)) = set
                .join_next()
                .await
                .transpose()
                .change_context(AnalyticsError::UnknownError)?
            {
                logger::info!("Logging metric: {metric} Result: {:?}", data);
                for (id, value) in data? {
                    let metrics_builder = metrics_accumulator.entry(id).or_default();
                    match metric {
                        ActivePaymentsMetrics::ActivePayments => {
                            metrics_builder.active_payments.add_metrics_bucket(&value)
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
        } else {
            logger::error!("Merchant ID not present");
            Ok(MetricsResponse {
                query_data: vec![],
                meta_data: [AnalyticsMetadata {
                    current_time_range: req.time_range,
                }],
            })
        }
    } else {
        logger::error!("Publishable key not present for merchant ID");
        Ok(MetricsResponse {
            query_data: vec![],
            meta_data: [AnalyticsMetadata {
                current_time_range: req.time_range,
            }],
        })
    }
}
