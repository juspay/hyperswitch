use std::collections::HashSet;

use api_models::analytics::{
    auth_events::AuthEventMetricsBucketIdentifier, Granularity, TimeRange,
};
use common_enums::AuthenticationStatus;
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::AuthEventMetricRow;
use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct AuthenticationAttemptCount;

#[async_trait::async_trait]
impl<T> super::AuthEventMetric<T> for AuthenticationAttemptCount
where
    T: AnalyticsDataSource + super::AuthEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>> {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::Authentications);

        query_builder
            .add_select_column(Aggregate::Count {
                field: None,
                alias: Some("count"),
            })
            .switch()?;

        query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;

        query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;

        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;

        query_builder
            .add_negative_filter_clause("authentication_status", AuthenticationStatus::Pending)
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        if let Some(granularity) = granularity {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        query_builder
            .execute_query::<AuthEventMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    AuthEventMetricsBucketIdentifier::new(i.time_bucket.clone()),
                    i,
                ))
            })
            .collect::<error_stack::Result<
                HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
