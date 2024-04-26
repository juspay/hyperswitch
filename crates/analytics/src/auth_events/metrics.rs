use api_models::analytics::{
    auth_events::{AuthEventMetrics, AuthEventMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, LoadRow, MetricsResult},
};

mod authentication_attempt_count;
mod authentication_success_count;
mod challenge_attempt_count;
mod challenge_flow_count;
mod challenge_success_count;
mod frictionless_flow_count;
mod three_ds_sdk_count;

use authentication_attempt_count::AuthenticationAttemptCount;
use authentication_success_count::AuthenticationSuccessCount;
use challenge_attempt_count::ChallengeAttemptCount;
use challenge_flow_count::ChallengeFlowCount;
use challenge_success_count::ChallengeSuccessCount;
use frictionless_flow_count::FrictionlessFlowCount;
use three_ds_sdk_count::ThreeDsSdkCount;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct AuthEventMetricRow {
    pub count: Option<i64>,
    pub time_bucket: Option<String>,
}

pub trait AuthEventMetricAnalytics: LoadRow<AuthEventMetricRow> {}

#[async_trait::async_trait]
pub trait AuthEventMetric<T>
where
    T: AnalyticsDataSource + AuthEventMetricAnalytics,
{
    async fn load_metrics(
        &self,
        merchant_id: &str,
        publishable_key: &str,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>>;
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
        merchant_id: &str,
        publishable_key: &str,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>> {
        match self {
            Self::ThreeDsSdkCount => {
                ThreeDsSdkCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::AuthenticationAttemptCount => {
                AuthenticationAttemptCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::AuthenticationSuccessCount => {
                AuthenticationSuccessCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::ChallengeFlowCount => {
                ChallengeFlowCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::ChallengeAttemptCount => {
                ChallengeAttemptCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::ChallengeSuccessCount => {
                ChallengeSuccessCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
            Self::FrictionlessFlowCount => {
                FrictionlessFlowCount
                    .load_metrics(merchant_id, publishable_key, granularity, time_range, pool)
                    .await
            }
        }
    }
}
