use std::collections::HashSet;

use api_models::analytics::{
    auth_events::{
        AuthEventDimensions, AuthEventFilters, AuthEventMetrics, AuthEventMetricsBucketIdentifier,
    },
    Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod authentication_attempt_count;
mod authentication_count;
mod authentication_error_message;
mod authentication_funnel;
mod authentication_success_count;
mod challenge_attempt_count;
mod challenge_flow_count;
mod challenge_success_count;
mod frictionless_flow_count;
mod frictionless_success_count;

use authentication_attempt_count::AuthenticationAttemptCount;
use authentication_count::AuthenticationCount;
use authentication_error_message::AuthenticationErrorMessage;
use authentication_funnel::AuthenticationFunnel;
use authentication_success_count::AuthenticationSuccessCount;
use challenge_attempt_count::ChallengeAttemptCount;
use challenge_flow_count::ChallengeFlowCount;
use challenge_success_count::ChallengeSuccessCount;
use frictionless_flow_count::FrictionlessFlowCount;
use frictionless_success_count::FrictionlessSuccessCount;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, Hash)]
pub struct AuthEventMetricRow {
    pub count: Option<i64>,
    pub authentication_status: Option<DBEnumWrapper<storage_enums::AuthenticationStatus>>,
    pub trans_status: Option<DBEnumWrapper<storage_enums::TransactionStatus>>,
    pub authentication_type: Option<DBEnumWrapper<storage_enums::DecoupledAuthenticationType>>,
    pub error_message: Option<String>,
    pub authentication_connector: Option<DBEnumWrapper<storage_enums::AuthenticationConnectors>>,
    pub message_version: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait AuthEventMetricAnalytics: LoadRow<AuthEventMetricRow> {}

#[async_trait::async_trait]
pub trait AuthEventMetric<T>
where
    T: AnalyticsDataSource + AuthEventMetricAnalytics,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dimensions: &[AuthEventDimensions],
        filters: &AuthEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> AuthEventMetric<T> for AuthEventMetrics
where
    T: AnalyticsDataSource + AuthEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dimensions: &[AuthEventDimensions],
        filters: &AuthEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>> {
        match self {
            Self::AuthenticationCount => {
                AuthenticationCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::AuthenticationAttemptCount => {
                AuthenticationAttemptCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::AuthenticationSuccessCount => {
                AuthenticationSuccessCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::ChallengeFlowCount => {
                ChallengeFlowCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::ChallengeAttemptCount => {
                ChallengeAttemptCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::ChallengeSuccessCount => {
                ChallengeSuccessCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::FrictionlessFlowCount => {
                FrictionlessFlowCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::FrictionlessSuccessCount => {
                FrictionlessSuccessCount
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::AuthenticationErrorMessage => {
                AuthenticationErrorMessage
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::AuthenticationFunnel => {
                AuthenticationFunnel
                    .load_metrics(
                        merchant_id,
                        dimensions,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
        }
    }
}
