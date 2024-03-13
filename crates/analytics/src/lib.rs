mod clickhouse;
pub mod core;
pub mod disputes;
pub mod errors;
pub mod metrics;
pub mod payments;
mod query;
pub mod refunds;

pub mod api_event;
pub mod connector_events;
pub mod health_check;
pub mod opensearch;
pub mod outgoing_webhook_event;
pub mod sdk_events;
pub mod search;
mod sqlx;
mod types;
use api_event::metrics::{ApiEventMetric, ApiEventMetricRow};
use common_utils::errors::CustomResult;
use disputes::metrics::{DisputeMetric, DisputeMetricRow};
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
pub use types::AnalyticsDomain;
pub mod lambda_utils;
pub mod utils;

use std::sync::Arc;

use api_models::analytics::{
    api_event::{
        ApiEventDimensions, ApiEventFilters, ApiEventMetrics, ApiEventMetricsBucketIdentifier,
    },
    disputes::{DisputeDimensions, DisputeFilters, DisputeMetrics, DisputeMetricsBucketIdentifier},
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    refunds::{RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier},
    sdk_events::{
        SdkEventDimensions, SdkEventFilters, SdkEventMetrics, SdkEventMetricsBucketIdentifier,
    },
    Distribution, Granularity, TimeRange,
};
use clickhouse::ClickhouseClient;
pub use clickhouse::ClickhouseConfig;
use error_stack::IntoReport;
use router_env::{
    logger,
    tracing::{self, instrument},
};
use storage_impl::config::Database;

use self::{
    payments::{
        distribution::{PaymentDistribution, PaymentDistributionRow},
        metrics::{PaymentMetric, PaymentMetricRow},
    },
    refunds::metrics::{RefundMetric, RefundMetricRow},
    sdk_events::metrics::{SdkEventMetric, SdkEventMetricRow},
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

impl Default for AnalyticsProvider {
    fn default() -> Self {
        Self::Sqlx(SqlxClient::default())
    }
}

impl ToString for AnalyticsProvider {
    fn to_string(&self) -> String {
        String::from(match self {
            Self::Clickhouse(_) => "Clickhouse",
            Self::Sqlx(_) => "Sqlx",
            Self::CombinedCkh(_, _) => "CombinedCkh",
            Self::CombinedSqlx(_, _) => "CombinedSqlx",
        })
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

    pub async fn get_payment_distribution(
        &self,
        distribution: &Distribution,
        dimensions: &[PaymentDimensions],
        merchant_id: &str,
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>> {
        // Metrics to get the fetch time for each payment metric
        metrics::request::record_operation_time(
            async {
                match self {
                        Self::Sqlx(pool) => {
                        distribution.distribution_for
                            .load_distribution(
                                distribution,
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
                        distribution.distribution_for
                            .load_distribution(
                                distribution,
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
                        let (ckh_result, sqlx_result) = tokio::join!(distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics distribution")
                            },
                            _ => {}

                        };

                        ckh_result
                    }
                                    Self::CombinedSqlx(sqlx_pool, ckh_pool) => {
                        let (ckh_result, sqlx_result) = tokio::join!(distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                merchant_id,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics distribution")
                            },
                            _ => {}

                        };

                        sqlx_result
                    }
                }
            },
            &metrics::METRIC_FETCH_TIME,
            &distribution.distribution_for,
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

    pub async fn get_dispute_metrics(
        &self,
        metric: &DisputeMetrics,
        dimensions: &[DisputeDimensions],
        merchant_id: &str,
        filters: &DisputeFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(DisputeMetricsBucketIdentifier, DisputeMetricRow)>> {
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
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics metrics")
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
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics metrics")
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

    pub async fn get_sdk_event_metrics(
        &self,
        metric: &SdkEventMetrics,
        dimensions: &[SdkEventDimensions],
        pub_key: &str,
        filters: &SdkEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(SdkEventMetricsBucketIdentifier, SdkEventMetricRow)>> {
        match self {
            Self::Sqlx(_pool) => Err(MetricsError::NotImplemented).into_report(),
            Self::Clickhouse(pool) => {
                metric
                    .load_metrics(dimensions, pub_key, filters, granularity, time_range, pool)
                    .await
            }
            Self::CombinedCkh(_sqlx_pool, ckh_pool) | Self::CombinedSqlx(_sqlx_pool, ckh_pool) => {
                metric
                    .load_metrics(
                        dimensions,
                        pub_key,
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

    pub async fn get_api_event_metrics(
        &self,
        metric: &ApiEventMetrics,
        dimensions: &[ApiEventDimensions],
        pub_key: &str,
        filters: &ApiEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>> {
        match self {
            Self::Sqlx(_pool) => Err(MetricsError::NotImplemented).into_report(),
            Self::Clickhouse(ckh_pool)
            | Self::CombinedCkh(_, ckh_pool)
            | Self::CombinedSqlx(_, ckh_pool) => {
                // Since API events are ckh only use ckh here
                metric
                    .load_metrics(
                        dimensions,
                        pub_key,
                        filters,
                        granularity,
                        time_range,
                        ckh_pool,
                    )
                    .await
            }
        }
    }

    pub async fn from_conf(config: &AnalyticsConfig) -> Self {
        match config {
            AnalyticsConfig::Sqlx { sqlx } => Self::Sqlx(SqlxClient::from_conf(sqlx).await),
            AnalyticsConfig::Clickhouse { clickhouse } => Self::Clickhouse(ClickhouseClient {
                config: Arc::new(clickhouse.clone()),
            }),
            AnalyticsConfig::CombinedCkh { sqlx, clickhouse } => Self::CombinedCkh(
                SqlxClient::from_conf(sqlx).await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                },
            ),
            AnalyticsConfig::CombinedSqlx { sqlx, clickhouse } => Self::CombinedSqlx(
                SqlxClient::from_conf(sqlx).await,
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

#[async_trait::async_trait]
impl SecretsHandler for AnalyticsConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let analytics_config = value.get_inner();
        let decrypted_password = match analytics_config {
            // Todo: Perform kms decryption of clickhouse password
            Self::Clickhouse { .. } => masking::Secret::new(String::default()),
            Self::Sqlx { sqlx }
            | Self::CombinedCkh { sqlx, .. }
            | Self::CombinedSqlx { sqlx, .. } => {
                secret_management_client
                    .get_secret(sqlx.password.clone())
                    .await?
            }
        };

        Ok(value.transition_state(|conf| match conf {
            Self::Sqlx { sqlx } => Self::Sqlx {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
            },
            Self::Clickhouse { clickhouse } => Self::Clickhouse { clickhouse },
            Self::CombinedCkh { sqlx, clickhouse } => Self::CombinedCkh {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
                clickhouse,
            },
            Self::CombinedSqlx { sqlx, clickhouse } => Self::CombinedSqlx {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
                clickhouse,
            },
        }))
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self::Sqlx {
            sqlx: Database::default(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, Default, serde::Serialize)]
pub struct ReportConfig {
    pub payment_function: String,
    pub refund_function: String,
    pub dispute_function: String,
    pub region: String,
}
