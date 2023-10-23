mod clickhouse;
pub mod core;
pub mod errors;
pub mod metrics;
pub mod payments;
mod query;
pub mod refunds;

// TODO: We can't declare routes here since authentication is defined in router_vas crate
// completely move this here once authentication dependencies are resolved
// https://github.com/juspay/hyperswitch-cloud/issues/660
// pub mod routes;

pub mod api_event;
pub mod sdk_events;
mod sqlx;
mod types;
pub use types::AnalyticsDomain;
pub mod lambda_utils;
pub mod utils;

use std::sync::Arc;

use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    refunds::{RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use clickhouse::ClickhouseClient;
pub use clickhouse::ClickhouseConfig;
use error_stack::IntoReport;
use hyperswitch_oss::{configs::settings::Database, logger};
use router_env::tracing::{self, instrument};

use self::{
    payments::metrics::{PaymentMetric, PaymentMetricRow},
    refunds::metrics::{RefundMetric, RefundMetricRow},
    sqlx::SqlxClient,
    types::MetricsError,
};
#[derive(Clone, Debug)]
pub enum AnalyticsProvider {
    Sqlx(SqlxClient),
    Clickhouse(ClickhouseClient),
    CombinedCkh(SqlxClient, ClickhouseClient),
    CombinedSqlx(SqlxClient, ClickhouseClient),
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
                                        Self::Clickhouse(pool) => {
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
                                    Self::CombinedCkh(sqlx_pool, ckh_pool) => {
                        let (ckh_result, sqlx_result) = tokio::join!(metric
                            .load_metrics(
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics metrics")
                            },
                            _ => {}

                        };

                        ckh_result
                    }
                                    Self::CombinedSqlx(sqlx_pool, ckh_pool) => {
                        let (ckh_result, sqlx_result) = tokio::join!(metric
                            .load_metrics(
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics metrics")
                            },
                            _ => {}

                        };

                        sqlx_result
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
        // Metrics to get the fetch time for each refund metric
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
                            Self::Clickhouse(pool) => {
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
                            Self::CombinedCkh(sqlx_pool, ckh_pool) => {
                                let (ckh_result, sqlx_result) = tokio::join!(
                                    metric.load_metrics(
                                        dimensions,
                                        merchant_id,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        merchant_id,
                                        filters,
                                        granularity,
                                        time_range,
                                        sqlx_pool,
                                    )
                                );
                                match (&sqlx_result, &ckh_result) {
                                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics metrics")
                                    }
                                    _ => {}
                                };
                                ckh_result
                            }
                            Self::CombinedSqlx(sqlx_pool, ckh_pool) => {
                                let (ckh_result, sqlx_result) = tokio::join!(
                                    metric.load_metrics(
                                        dimensions,
                                        merchant_id,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        merchant_id,
                                        filters,
                                        granularity,
                                        time_range,
                                        sqlx_pool,
                                    )
                                );
                                match (&sqlx_result, &ckh_result) {
                                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics metrics")
                                    }
                                    _ => {}
                                };
                                sqlx_result
                            }
                        }
                    },
                   &metrics::METRIC_FETCH_TIME,
       metric,
            self,
        )
        .await
    }

    pub async fn from_conf(
        config: &AnalyticsConfig,
        #[cfg(feature = "kms")] kms_client: &external_services_oss::kms::KmsClient,
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
            AnalyticsConfig::Clickhouse { clickhouse } => Self::Clickhouse(ClickhouseClient {
                config: Arc::new(clickhouse.clone()),
            }),
            AnalyticsConfig::CombinedCkh { sqlx, clickhouse } => Self::CombinedCkh(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_client,
                )
                .await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                },
            ),
            AnalyticsConfig::CombinedSqlx { sqlx, clickhouse } => Self::CombinedSqlx(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_client,
                )
                .await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                },
            ),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "source")]
#[serde(rename_all = "lowercase")]
pub enum AnalyticsConfig {
    Sqlx {
        sqlx: Database,
    },
    Clickhouse {
        clickhouse: ClickhouseConfig,
    },
    CombinedCkh {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
    },
    CombinedSqlx {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
    },
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self::Sqlx {
            sqlx: Database::default(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, Default, serde::Serialize)]
pub struct PaymentReportConfig {
    pub function_name: String,
    pub region: String,
}
