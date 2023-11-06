mod core;
mod errors;
pub mod metrics;
mod payments;
mod query;
mod refunds;
pub mod routes;

mod sqlx;
mod types;
mod utils;

use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    refunds::{RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use router_env::{instrument, tracing};

use self::{
    payments::metrics::{PaymentMetric, PaymentMetricRow},
    refunds::metrics::{RefundMetric, RefundMetricRow},
    sqlx::SqlxClient,
};
use crate::configs::settings::Database;

#[derive(Clone, Debug)]
pub enum AnalyticsProvider {
    Sqlx(SqlxClient),
}

impl Default for AnalyticsProvider {
    fn default() -> Self {
        Self::Sqlx(SqlxClient::default())
    }
}

impl AnalyticsProvider {
    #[instrument(skip_all)]
    pub async fn get_payment_metrics(
        &self,
        metric: &PaymentMetrics,
        dimensions: &[PaymentDimensions],
        merchant_id: &str,
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        // Metrics to get the fetch time for each payment metric
        metrics::request::record_operation_time(
            async {
                match self {
                    Self::Sqlx(pool) => {
                        metric
                            .load_metrics(
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                pool,
                            )
                            .await
                    }
                }
            },
            &metrics::METRIC_FETCH_TIME,
            metric,
            self,
        )
        .await
    }

    pub async fn get_refund_metrics(
        &self,
        metric: &RefundMetrics,
        dimensions: &[RefundDimensions],
        merchant_id: &str,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundMetricRow)>> {
        match self {
            Self::Sqlx(pool) => {
                metric
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
        }
    }

    pub async fn from_conf(
        config: &AnalyticsConfig,
        #[cfg(feature = "kms")] kms_client: &external_services::kms::KmsClient,
    ) -> Self {
        match config {
            AnalyticsConfig::Sqlx { sqlx } => Self::Sqlx(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_client,
                )
                .await,
            ),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "source")]
#[serde(rename_all = "lowercase")]
pub enum AnalyticsConfig {
    Sqlx { sqlx: Database },
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self::Sqlx {
            sqlx: Database::default(),
        }
    }
}
