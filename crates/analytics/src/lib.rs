pub mod active_payments;
pub mod api_event;
pub mod auth_events;
mod clickhouse;
pub mod connector_events;
pub mod core;
pub mod disputes;
pub mod enums;
pub mod errors;
pub mod frm;
pub mod health_check;
pub mod metrics;
pub mod opensearch;
pub mod outgoing_webhook_event;
pub mod payment_intents;
pub mod payments;
mod query;
pub mod refunds;
pub mod routing_events;
pub mod sdk_events;
pub mod search;
mod sqlx;
mod types;
use api_event::metrics::{ApiEventMetric, ApiEventMetricRow};
use common_utils::{errors::CustomResult, types::TenantConfig};
use disputes::metrics::{DisputeMetric, DisputeMetricRow};
use enums::AuthInfo;
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use refunds::distribution::{RefundDistribution, RefundDistributionRow};
pub use types::AnalyticsDomain;
pub mod lambda_utils;
pub mod utils;

use std::{collections::HashSet, sync::Arc};

use api_models::analytics::{
    active_payments::{ActivePaymentsMetrics, ActivePaymentsMetricsBucketIdentifier},
    api_event::{
        ApiEventDimensions, ApiEventFilters, ApiEventMetrics, ApiEventMetricsBucketIdentifier,
    },
    auth_events::{
        AuthEventDimensions, AuthEventFilters, AuthEventMetrics, AuthEventMetricsBucketIdentifier,
    },
    disputes::{DisputeDimensions, DisputeFilters, DisputeMetrics, DisputeMetricsBucketIdentifier},
    frm::{FrmDimensions, FrmFilters, FrmMetrics, FrmMetricsBucketIdentifier},
    payment_intents::{
        PaymentIntentDimensions, PaymentIntentFilters, PaymentIntentMetrics,
        PaymentIntentMetricsBucketIdentifier,
    },
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    refunds::{RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier},
    sdk_events::{
        SdkEventDimensions, SdkEventFilters, SdkEventMetrics, SdkEventMetricsBucketIdentifier,
    },
    Granularity, PaymentDistributionBody, RefundDistributionBody, TimeRange,
};
use clickhouse::ClickhouseClient;
pub use clickhouse::ClickhouseConfig;
use error_stack::report;
use masking::PeekInterface;
use router_env::{
    logger,
    tracing::{self, instrument},
    types::FlowMetric,
};
use storage_impl::config::Database;
use strum::Display;

use self::{
    active_payments::metrics::{ActivePaymentsMetric, ActivePaymentsMetricRow},
    auth_events::metrics::{AuthEventMetric, AuthEventMetricRow},
    frm::metrics::{FrmMetric, FrmMetricRow},
    opensearch::{OpenSearchClient, OpenSearchConfig},
    payment_intents::metrics::{PaymentIntentMetric, PaymentIntentMetricRow},
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

#[derive(Clone, Debug)]
pub enum SearchProvider {
    Sqlx(SqlxClient),
    Opensearch(OpenSearchClient),
    CombinedOpensearch(SqlxClient, OpenSearchClient),
    CombinedSqlx(SqlxClient, OpenSearchClient),
}

impl Default for AnalyticsProvider {
    fn default() -> Self {
        Self::Sqlx(SqlxClient::default())
    }
}

impl Default for SearchProvider {
    fn default() -> Self {
        Self::Sqlx(SqlxClient::default())
    }
}

impl std::fmt::Display for AnalyticsProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let analytics_provider = match self {
            Self::Clickhouse(_) => "Clickhouse",
            Self::Sqlx(_) => "Sqlx",
            Self::CombinedCkh(_, _) => "CombinedCkh",
            Self::CombinedSqlx(_, _) => "CombinedSqlx",
        };

        write!(f, "{analytics_provider}")
    }
}

impl std::fmt::Display for SearchProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let search_provider = match self {
            Self::Opensearch(_) => "Opensearch",
            Self::Sqlx(_) => "Sqlx",
            Self::CombinedOpensearch(_, _) => "CombinedOpensearch",
            Self::CombinedSqlx(_, _) => "CombinedSqlx",
        };

        write!(f, "{search_provider}")
    }
}

impl AnalyticsProvider {
    #[instrument(skip_all)]
    pub async fn get_payment_metrics(
        &self,
        metric: &PaymentMetrics,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        // Metrics to get the fetch time for each payment metric
        metrics::request::record_operation_time(
            async {
                match self {
                        Self::Sqlx(pool) => {
                        metric
                            .load_metrics(
                                dimensions,
                                auth,
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
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                auth,
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
        distribution: &PaymentDistributionBody,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
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
                                auth,
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
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                auth,
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

    pub async fn get_payment_intent_metrics(
        &self,
        metric: &PaymentIntentMetrics,
        dimensions: &[PaymentIntentDimensions],
        auth: &AuthInfo,
        filters: &PaymentIntentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>>
    {
        // Metrics to get the fetch time for each payment intent metric
        metrics::request::record_operation_time(
            async {
                match self {
                        Self::Sqlx(pool) => {
                        metric
                            .load_metrics(
                                dimensions,
                                auth,
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
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                auth,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payment intents analytics metrics")
                            },
                            _ => {}

                        };

                        ckh_result
                    }
                                    Self::CombinedSqlx(sqlx_pool, ckh_pool) => {
                        let (ckh_result, sqlx_result) = tokio::join!(metric
                            .load_metrics(
                                dimensions,
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            metric
                            .load_metrics(
                                dimensions,
                                auth,
                                filters,
                                granularity,
                                time_range,
                                sqlx_pool,
                            ));
                        match (&sqlx_result, &ckh_result) {
                            (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                                router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payment intents analytics metrics")
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
        auth: &AuthInfo,
        filters: &RefundFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(RefundMetricsBucketIdentifier, RefundMetricRow)>> {
        // Metrics to get the fetch time for each refund metric
        metrics::request::record_operation_time(
            async {
                        match self {
                            Self::Sqlx(pool) => {
                                metric
                                    .load_metrics(
                                        dimensions,
                                        auth,
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
                                        auth,
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
                                        auth,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        auth,
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
                                        auth,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        auth,
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

    pub async fn get_refund_distribution(
        &self,
        distribution: &RefundDistributionBody,
        dimensions: &[RefundDimensions],
        auth: &AuthInfo,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundDistributionRow)>> {
        // Metrics to get the fetch time for each payment metric
        metrics::request::record_operation_time(
            async {
                match self {
                        Self::Sqlx(pool) => {
                        distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                auth,
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
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                auth,
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
                                auth,
                                filters,
                                granularity,
                                time_range,
                                ckh_pool,
                            ),
                            distribution.distribution_for
                            .load_distribution(
                                distribution,
                                dimensions,
                                auth,
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

    pub async fn get_frm_metrics(
        &self,
        metric: &FrmMetrics,
        dimensions: &[FrmDimensions],
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &FrmFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<Vec<(FrmMetricsBucketIdentifier, FrmMetricRow)>> {
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
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres frm analytics metrics")
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
                                        logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres frm analytics metrics")
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
        auth: &AuthInfo,
        filters: &DisputeFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(DisputeMetricsBucketIdentifier, DisputeMetricRow)>> {
        // Metrics to get the fetch time for each refund metric
        metrics::request::record_operation_time(
            async {
                        match self {
                            Self::Sqlx(pool) => {
                                metric
                                    .load_metrics(
                                        dimensions,
                                        auth,
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
                                        auth,
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
                                        auth,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        auth,
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
                                        auth,
                                        filters,
                                        granularity,
                                        time_range,
                                        ckh_pool,
                                    ),
                                    metric.load_metrics(
                                        dimensions,
                                        auth,
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
        publishable_key: &str,
        filters: &SdkEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(SdkEventMetricsBucketIdentifier, SdkEventMetricRow)>> {
        match self {
            Self::Sqlx(_pool) => Err(report!(MetricsError::NotImplemented)),
            Self::Clickhouse(pool) => {
                metric
                    .load_metrics(
                        dimensions,
                        publishable_key,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::CombinedCkh(_sqlx_pool, ckh_pool) | Self::CombinedSqlx(_sqlx_pool, ckh_pool) => {
                metric
                    .load_metrics(
                        dimensions,
                        publishable_key,
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

    pub async fn get_active_payments_metrics(
        &self,
        metric: &ActivePaymentsMetrics,
        merchant_id: &common_utils::id_type::MerchantId,
        publishable_key: &str,
        time_range: &TimeRange,
    ) -> types::MetricsResult<
        HashSet<(
            ActivePaymentsMetricsBucketIdentifier,
            ActivePaymentsMetricRow,
        )>,
    > {
        match self {
            Self::Sqlx(_pool) => Err(report!(MetricsError::NotImplemented)),
            Self::Clickhouse(pool) => {
                metric
                    .load_metrics(merchant_id, publishable_key, time_range, pool)
                    .await
            }
            Self::CombinedCkh(_sqlx_pool, ckh_pool) | Self::CombinedSqlx(_sqlx_pool, ckh_pool) => {
                metric
                    .load_metrics(merchant_id, publishable_key, time_range, ckh_pool)
                    .await
            }
        }
    }

    pub async fn get_auth_event_metrics(
        &self,
        metric: &AuthEventMetrics,
        dimensions: &[AuthEventDimensions],
        auth: &AuthInfo,
        filters: &AuthEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>> {
        match self {
            Self::Sqlx(_pool) => Err(report!(MetricsError::NotImplemented)),
            Self::Clickhouse(pool) => {
                metric
                    .load_metrics(auth, dimensions, filters, granularity, time_range, pool)
                    .await
            }
            Self::CombinedCkh(_sqlx_pool, ckh_pool) | Self::CombinedSqlx(_sqlx_pool, ckh_pool) => {
                metric
                    .load_metrics(
                        auth,
                        dimensions,
                        filters,
                        granularity,
                        // Since API events are ckh only use ckh here
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
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &ApiEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
    ) -> types::MetricsResult<HashSet<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>> {
        match self {
            Self::Sqlx(_pool) => Err(report!(MetricsError::NotImplemented)),
            Self::Clickhouse(ckh_pool)
            | Self::CombinedCkh(_, ckh_pool)
            | Self::CombinedSqlx(_, ckh_pool) => {
                // Since API events are ckh only use ckh here
                metric
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        ckh_pool,
                    )
                    .await
            }
        }
    }

    pub async fn from_conf(config: &AnalyticsConfig, tenant: &dyn TenantConfig) -> Self {
        match config {
            AnalyticsConfig::Sqlx { sqlx, .. } => {
                Self::Sqlx(SqlxClient::from_conf(sqlx, tenant.get_schema()).await)
            }
            AnalyticsConfig::Clickhouse { clickhouse, .. } => Self::Clickhouse(ClickhouseClient {
                config: Arc::new(clickhouse.clone()),
                database: tenant.get_clickhouse_database().to_string(),
            }),
            AnalyticsConfig::CombinedCkh {
                sqlx, clickhouse, ..
            } => Self::CombinedCkh(
                SqlxClient::from_conf(sqlx, tenant.get_schema()).await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                    database: tenant.get_clickhouse_database().to_string(),
                },
            ),
            AnalyticsConfig::CombinedSqlx {
                sqlx, clickhouse, ..
            } => Self::CombinedSqlx(
                SqlxClient::from_conf(sqlx, tenant.get_schema()).await,
                ClickhouseClient {
                    config: Arc::new(clickhouse.clone()),
                    database: tenant.get_clickhouse_database().to_string(),
                },
            ),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum AnalyticsConfig {
    Sqlx {
        sqlx: Database,
        #[serde(default)]
        forex_enabled: bool,
    },
    Clickhouse {
        clickhouse: ClickhouseConfig,
        #[serde(default)]
        forex_enabled: bool,
    },
    CombinedCkh {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
        #[serde(default)]
        forex_enabled: bool,
    },
    CombinedSqlx {
        sqlx: Database,
        clickhouse: ClickhouseConfig,
        #[serde(default)]
        forex_enabled: bool,
    },
}

impl AnalyticsConfig {
    pub fn get_forex_enabled(&self) -> bool {
        match self {
            Self::Sqlx { forex_enabled, .. }
            | Self::Clickhouse { forex_enabled, .. }
            | Self::CombinedCkh { forex_enabled, .. }
            | Self::CombinedSqlx { forex_enabled, .. } => *forex_enabled,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SearchConfig {
    Sqlx {
        sqlx: Database,
    },
    Opensearch {
        opensearch: OpenSearchConfig,
    },
    CombinedOpensearch {
        sqlx: Database,
        opensearch: OpenSearchConfig,
    },
    CombinedSqlx {
        sqlx: Database,
        opensearch: OpenSearchConfig,
    },
}

impl<'de> serde::Deserialize<'de> for SearchConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(serde::Deserialize)]
        struct SearchConfigHelper {
            #[serde(default)]
            opensearch: Option<OpenSearchConfig>,
            #[serde(default)]
            sqlx: Option<Database>,
            #[serde(default)]
            source: Option<String>,
        }

        let helper = SearchConfigHelper::deserialize(deserializer)?;

        // If source field exists (old format), use it for backwards compatibility
        if let Some(source) = helper.source {
            return match source.as_str() {
                "opensearch" => {
                    let opensearch = helper.opensearch.ok_or_else(|| {
                        D::Error::custom("opensearch config required when source=opensearch")
                    })?;
                    Ok(Self::Opensearch { opensearch })
                }
                "sqlx" => {
                    let sqlx = helper
                        .sqlx
                        .ok_or_else(|| D::Error::custom("sqlx config required when source=sqlx"))?;
                    Ok(Self::Sqlx { sqlx })
                }
                _ => Err(D::Error::custom(format!("unknown source: {}", source))),
            };
        }

        // New format: automatically detect from presence of fields
        match (helper.opensearch, helper.sqlx) {
            (Some(opensearch), Some(sqlx)) => {
                // Both present - use CombinedOpensearch if opensearch is enabled
                if opensearch.enabled {
                    Ok(Self::CombinedOpensearch { sqlx, opensearch })
                } else {
                    Ok(Self::Sqlx { sqlx })
                }
            }
            (Some(opensearch), None) => {
                // Only opensearch present
                if opensearch.enabled {
                    Ok(Self::Opensearch { opensearch })
                } else {
                    // OpenSearch disabled, fall back to default
                    logger::warn!("OpenSearch is disabled, falling back to default");
                    Ok(Self::default())
                }
            }
            (None, Some(sqlx)) => {
                // Only sqlx present
                logger::debug!("Only sqlx present");
                Ok(Self::Sqlx { sqlx })
            }
            (None, None) => {
                // Neither present, use default
                logger::warn!("Neither opensearch nor sqlx present in config, using default");
                Ok(Self::default())
            }
        }
    }
}

impl SearchConfig {
    pub fn should_use_opensearch(&self) -> bool {
        match self {
            Self::Sqlx { .. } => false,
            Self::Opensearch { .. }
            | Self::CombinedOpensearch { .. }
            | Self::CombinedSqlx { .. } => true,
        }
    }

    pub fn validate(&self) -> Result<(), storage_impl::errors::ApplicationError> {
        match self {
            Self::Sqlx { .. } => Ok(()),
            Self::Opensearch { opensearch }
            | Self::CombinedOpensearch { opensearch, .. }
            | Self::CombinedSqlx { opensearch, .. } => opensearch.validate(),
        }
    }
}

#[async_trait::async_trait]
impl health_check::HealthCheck for SearchProvider {
    async fn deep_health_check(&self) -> CustomResult<(), types::QueryExecutionError> {
        match self {
            Self::Sqlx(client) => client.deep_health_check().await,
            Self::Opensearch(client) => client.deep_health_check().await,
            Self::CombinedOpensearch(sqlx_client, opensearch_client)
            | Self::CombinedSqlx(sqlx_client, opensearch_client) => {
                sqlx_client.deep_health_check().await?;
                opensearch_client.deep_health_check().await
            }
        }
    }
}

impl SearchProvider {
    pub async fn search_results(
        &self,
        req: api_models::analytics::search::GetSearchRequestWithIndex,
        auth: Vec<AuthInfo>,
    ) -> CustomResult<api_models::analytics::search::GetSearchResponse, opensearch::OpenSearchError>
    {
        match self {
            Self::Opensearch(client) => search::search_results(client, req, auth).await,
            Self::Sqlx(_) => Err(error_stack::report!(
                opensearch::OpenSearchError::NotImplemented
            )),
            Self::CombinedOpensearch(_sqlx_client, opensearch_client) => {
                let os_res = search::search_results(opensearch_client, req, auth).await;
                // Currently returning OS result, SQLx logic can be added later for comparison
                os_res
            }
            Self::CombinedSqlx(_sqlx_client, opensearch_client) => {
                let os_res = search::search_results(opensearch_client, req, auth).await;
                // Currently returning OS result, SQLx logic can be added later for comparison
                os_res
            }
        }
    }

    pub async fn msearch_results(
        &self,
        req: api_models::analytics::search::GetGlobalSearchRequest,
        search_params: Vec<AuthInfo>,
        indexes: Vec<api_models::analytics::search::SearchIndex>,
    ) -> CustomResult<
        Vec<api_models::analytics::search::GetSearchResponse>,
        opensearch::OpenSearchError,
    > {
        match self {
            Self::Opensearch(client) => {
                search::msearch_results(client, req, search_params, indexes).await
            }
            Self::Sqlx(_) => Err(error_stack::report!(
                opensearch::OpenSearchError::NotImplemented
            )),
            Self::CombinedOpensearch(_sqlx_client, opensearch_client) => {
                let os_res =
                    search::msearch_results(opensearch_client, req, search_params, indexes).await;
                os_res
            }
            Self::CombinedSqlx(_sqlx_client, opensearch_client) => {
                let os_res =
                    search::msearch_results(opensearch_client, req, search_params, indexes).await;
                os_res
            }
        }
    }

    pub fn is_opensearch_enabled(&self) -> bool {
        !matches!(self, Self::Sqlx(_))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for SearchConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let (sqlx_decrypted_password, opensearch_decrypted_password) = match value.get_inner() {
            Self::Sqlx { sqlx } => {
                let decrypted = secret_management_client
                    .get_secret(sqlx.password.clone())
                    .await?;
                (Some(decrypted), None)
            }
            Self::Opensearch { opensearch } => {
                let decrypted = match &opensearch.auth {
                    opensearch::OpenSearchAuth::Basic { password, .. } => Some(
                        secret_management_client
                            .get_secret(masking::Secret::new(password.clone()))
                            .await?,
                    ),
                    opensearch::OpenSearchAuth::Aws { .. } => None,
                };
                (None, decrypted)
            }
            Self::CombinedOpensearch { sqlx, opensearch }
            | Self::CombinedSqlx { sqlx, opensearch } => {
                let sqlx_decrypted = secret_management_client
                    .get_secret(sqlx.password.clone())
                    .await?;
                let opensearch_decrypted = match &opensearch.auth {
                    opensearch::OpenSearchAuth::Basic { password, .. } => Some(
                        secret_management_client
                            .get_secret(masking::Secret::new(password.clone()))
                            .await?,
                    ),
                    opensearch::OpenSearchAuth::Aws { .. } => None,
                };
                (Some(sqlx_decrypted), opensearch_decrypted)
            }
        };

        Ok(value.transition_state(|conf| match conf {
            Self::Sqlx { sqlx } => Self::Sqlx {
                sqlx: Database {
                    password: sqlx_decrypted_password.unwrap_or_default(),
                    ..sqlx
                },
            },
            Self::Opensearch { mut opensearch } => {
                if let (Some(decrypted), opensearch::OpenSearchAuth::Basic { username, .. }) =
                    (opensearch_decrypted_password, &opensearch.auth)
                {
                    opensearch.auth = opensearch::OpenSearchAuth::Basic {
                        username: username.clone(),
                        password: decrypted.peek().clone(),
                    };
                }
                Self::Opensearch { opensearch }
            }
            Self::CombinedOpensearch {
                sqlx,
                mut opensearch,
            } => {
                if let (Some(decrypted), opensearch::OpenSearchAuth::Basic { username, .. }) =
                    (opensearch_decrypted_password, &opensearch.auth)
                {
                    opensearch.auth = opensearch::OpenSearchAuth::Basic {
                        username: username.clone(),
                        password: decrypted.peek().clone(),
                    };
                }
                Self::CombinedOpensearch {
                    sqlx: Database {
                        password: sqlx_decrypted_password.unwrap_or_default(),
                        ..sqlx
                    },
                    opensearch,
                }
            }
            Self::CombinedSqlx {
                sqlx,
                mut opensearch,
            } => {
                if let (Some(decrypted), opensearch::OpenSearchAuth::Basic { username, .. }) =
                    (opensearch_decrypted_password, &opensearch.auth)
                {
                    opensearch.auth = opensearch::OpenSearchAuth::Basic {
                        username: username.clone(),
                        password: decrypted.peek().clone(),
                    };
                }
                Self::CombinedSqlx {
                    sqlx: Database {
                        password: sqlx_decrypted_password.unwrap_or_default(),
                        ..sqlx
                    },
                    opensearch,
                }
            }
        }))
    }
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
            Self::Sqlx { sqlx, .. }
            | Self::CombinedCkh { sqlx, .. }
            | Self::CombinedSqlx { sqlx, .. } => {
                secret_management_client
                    .get_secret(sqlx.password.clone())
                    .await?
            }
        };

        Ok(value.transition_state(|conf| match conf {
            Self::Sqlx {
                sqlx,
                forex_enabled,
            } => Self::Sqlx {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
                forex_enabled,
            },
            Self::Clickhouse {
                clickhouse,
                forex_enabled,
            } => Self::Clickhouse {
                clickhouse,
                forex_enabled,
            },
            Self::CombinedCkh {
                sqlx,
                clickhouse,
                forex_enabled,
            } => Self::CombinedCkh {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
                clickhouse,
                forex_enabled,
            },
            Self::CombinedSqlx {
                sqlx,
                clickhouse,
                forex_enabled,
            } => Self::CombinedSqlx {
                sqlx: Database {
                    password: decrypted_password,
                    ..sqlx
                },
                clickhouse,
                forex_enabled,
            },
        }))
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self::CombinedOpensearch {
            sqlx: Database::default(),
            opensearch: OpenSearchConfig::default(),
        }
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self::Sqlx {
            sqlx: Database::default(),
            forex_enabled: false,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, Default, serde::Serialize)]
pub struct ReportConfig {
    pub payment_function: String,
    pub refund_function: String,
    pub dispute_function: String,
    pub authentication_function: String,
    pub payout_function: String,
    pub region: String,
}

/// Analytics Flow routes Enums
/// Info - Dimensions and filters available for the domain
/// Filters - Set of values present for the dimension
/// Metrics - Analytical data on dimensions and metrics
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum AnalyticsFlow {
    GetInfo,
    GetPaymentMetrics,
    GetPaymentIntentMetrics,
    GetRefundsMetrics,
    GetFrmMetrics,
    GetSdkMetrics,
    GetAuthMetrics,
    GetAuthEventFilters,
    GetActivePaymentsMetrics,
    GetPaymentFilters,
    GetPaymentIntentFilters,
    GetRefundFilters,
    GetFrmFilters,
    GetSdkEventFilters,
    GetApiEvents,
    GetSdkEvents,
    GeneratePaymentReport,
    GenerateDisputeReport,
    GenerateRefundReport,
    GenerateAuthenticationReport,
    GeneratePayoutReport,
    GetApiEventMetrics,
    GetApiEventFilters,
    GetConnectorEvents,
    GetOutgoingWebhookEvents,
    GetGlobalSearchResults,
    GetSearchResults,
    GetDisputeFilters,
    GetDisputeMetrics,
    GetSankey,
    GetRoutingEvents,
}

impl FlowMetric for AnalyticsFlow {}
