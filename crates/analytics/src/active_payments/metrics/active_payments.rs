use std::collections::HashSet;

use api_models::analytics::{
    active_payments::ActivePaymentsMetricsBucketIdentifier, Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::ActivePaymentsMetricRow;
use crate::{
    query::{Aggregate, FilterTypes, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct ActivePayments;

#[async_trait::async_trait]
impl<T> super::ActivePaymentsMetric<T> for ActivePayments
where
    T: AnalyticsDataSource + super::ActivePaymentsMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        publishable_key: &str,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<
        HashSet<(
            ActivePaymentsMetricsBucketIdentifier,
            ActivePaymentsMetricRow,
        )>,
    > {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::ActivePaymentsAnalytics);

        query_builder
            .add_select_column(Aggregate::DistinctCount {
                field: "payment_id",
                alias: Some("count"),
            })
            .switch()?;

        query_builder
            .add_custom_filter_clause(
                "merchant_id",
                format!("'{}','{}'", merchant_id.get_string_repr(), publishable_key),
                FilterTypes::In,
            )
            .switch()?;

        query_builder
            .add_negative_filter_clause("payment_id", "")
            .switch()?;

        query_builder
            .add_custom_filter_clause(
                "flow_type",
                "'sdk', 'payment', 'payment_redirection_response'",
                FilterTypes::In,
            )
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        query_builder
            .execute_query::<ActivePaymentsMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| Ok((ActivePaymentsMetricsBucketIdentifier::new(None), i)))
            .collect::<error_stack::Result<
                HashSet<(
                    ActivePaymentsMetricsBucketIdentifier,
                    ActivePaymentsMetricRow,
                )>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
