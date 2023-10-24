#[cfg(feature = "clickhouse_analytics")]
mod clickhouse;
mod core;
mod errors;
pub mod metrics;
mod payments;
mod query;
mod refunds;
pub mod routes;
#[cfg(feature = "clickhouse_analytics")]
mod sdk_events;
#[cfg(feature = "sqlx_analytics")]
mod sqlx;
mod types;
mod utils;

#[cfg(feature = "clickhouse_analytics")]
use std::sync::Arc;

#[cfg(feature = "clickhouse_analytics")]
use api_models::analytics::sdk_events::{
    SdkEventDimensions, SdkEventFilters, SdkEventMetrics, SdkEventMetricsBucketIdentifier,
};
use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    refunds::{RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier},
    Granularity, TimeRange,
};
#[cfg(feature = "clickhouse_analytics")]
use clickhouse::ClickhouseClient;
#[cfg(feature = "clickhouse_analytics")]
pub use clickhouse::ClickhouseConfig;
#[cfg(all(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use error_stack::IntoReport;
#[cfg(feature = "sqlx_analytics")]
use hyperswitch_oss::configs::settings::Database;

#[cfg(feature = "clickhouse_analytics")]
use self::sdk_events::metrics::SdkEventMetric;
#[cfg(feature = "clickhouse_analytics")]
use self::sdk_events::metrics::SdkEventMetricRow;
#[cfg(feature = "sqlx_analytics")]
use self::sqlx::SqlxClient;
#[cfg(all(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use self::types::MetricsError;
use self::{
    payments::metrics::{PaymentMetric, PaymentMetricRow},
    refunds::metrics::{RefundMetric, RefundMetricRow},
};

#[derive(Clone, Debug)]
pub enum AnalyticsProvider {
    #[cfg(feature = "sqlx_analytics")]
    Sqlx(SqlxClient),
    #[cfg(feature = "clickhouse_analytics")]
    Clickhouse(ClickhouseClient),
    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
    CombinedCkh(SqlxClient, ClickhouseClient),
    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
    CombinedSqlx(SqlxClient, ClickhouseClient),
}
use router_env::{instrument, tracing};

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
                    #[cfg(feature = "sqlx_analytics")]
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
                    #[cfg(feature = "clickhouse_analytics")]
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
                    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
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
                                router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics metrics")
                            },
                            _ => {}

                        };

                        ckh_result
                    }
                    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
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
                                router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics metrics")
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
        match self {
            #[cfg(feature = "sqlx_analytics")]
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
            #[cfg(feature = "clickhouse_analytics")]
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
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
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
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics metrics")
                    }
                    _ => {}
                };
                ckh_result
            }
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
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
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics metrics")
                    }
                    _ => {}
                };
                sqlx_result
            }
        }
    }

    #[cfg(feature = "clickhouse_analytics")]
    pub async fn get_sdk_event_metrics(
        &self,
        metric: &SdkEventMetrics,
        dimensions: &[SdkEventDimensions],
        merchant_id: &str,
        filters: &SdkEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(SdkEventMetricsBucketIdentifier, SdkEventMetricRow)>> {
        match self {
            #[cfg(feature = "sqlx_analytics")]
            Self::Sqlx(_pool) => Err(MetricsError::NotImplemented).into_report(),
            #[cfg(feature = "clickhouse_analytics")]
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
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            Self::CombinedCkh(_sqlx_pool, ckh_pool) | Self::CombinedSqlx(_sqlx_pool, ckh_pool) => {
                metric
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        // Since SDK events are ckh only use ckh here
                        time_range,
                        ckh_pool,
                    )
                    .await
            }
        }
    }

    pub async fn from_conf(
        config: &AnalyticsConfig,
        #[cfg(feature = "kms")] kms_conf: &external_services_oss::kms::KmsConfig,
    ) -> Self {
        match config {
            #[cfg(feature = "sqlx_analytics")]
            AnalyticsConfig::Sqlx { sqlx } => Self::Sqlx(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_conf,
                )
                .await,
            ),
            #[cfg(feature = "clickhouse_analytics")]
            AnalyticsConfig::Clickhouse { clickhouse } => Self::Clickhouse(ClickhouseClient {
                config: Arc::new(clickhouse.clone()),
            }),
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsConfig::CombinedCkh { sqlx, clickhouse } => Self::CombinedCkh(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_conf,
                )
                .await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                },
            ),
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsConfig::CombinedSqlx { sqlx, clickhouse } => Self::CombinedSqlx(
                SqlxClient::from_conf(
                    sqlx,
                    #[cfg(feature = "kms")]
                    kms_conf,
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
    #[cfg(feature = "sqlx_analytics")]
    Sqlx { sqlx: Database },
    #[cfg(feature = "clickhouse_analytics")]
    Clickhouse { clickhouse: ClickhouseConfig },
    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
    CombinedCkh {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
    },
    #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
    CombinedSqlx {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
    },
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        #[cfg(feature = "sqlx_analytics")]
        return Self::Sqlx {
            sqlx: Database::default(),
        };
        #[cfg(not(feature = "sqlx_analytics"))]
        return Self::Clickhouse {
            clickhouse: ClickhouseConfig::default(),
        };
    }
}
