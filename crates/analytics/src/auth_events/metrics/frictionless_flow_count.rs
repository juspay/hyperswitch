use std::collections::HashSet;

use api_models::analytics::{
    auth_events::{AuthEventDimensions, AuthEventFilters, AuthEventMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_enums::DecoupledAuthenticationType;
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::AuthEventMetricRow;
use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
    AuthInfo,
};

#[derive(Default)]
pub(super) struct FrictionlessFlowCount;

#[async_trait::async_trait]
impl<T> super::AuthEventMetric<T> for FrictionlessFlowCount
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
        auth: &AuthInfo,
        dimensions: &[AuthEventDimensions],
        filters: &AuthEventFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(AuthEventMetricsBucketIdentifier, AuthEventMetricRow)>> {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::Authentications);
        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }
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
            .add_filter_clause(
                "authentication_type",
                DecoupledAuthenticationType::Frictionless,
            )
            .switch()?;
        filters.set_filter_clause(&mut query_builder).switch()?;
        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;
        auth.set_filter_clause(&mut query_builder).switch()?;

        for dim in dimensions.iter() {
            query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

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
                    AuthEventMetricsBucketIdentifier::new(
                        i.authentication_status.as_ref().map(|i| i.0),
                        i.trans_status.as_ref().map(|i| i.0.clone()),
                        i.authentication_type.as_ref().map(|i| i.0),
                        i.error_message.clone(),
                        i.authentication_connector.as_ref().map(|i| i.0),
                        i.message_version.clone(),
                        i.acs_reference_number.clone(),
                        i.mcc.clone(),
                        i.currency.as_ref().map(|i| i.0),
                        i.merchant_country.clone(),
                        i.billing_country.clone(),
                        i.shipping_country.clone(),
                        i.issuer_country.clone(),
                        i.earliest_supported_version.clone(),
                        i.latest_supported_version.clone(),
                        i.whitelist_decision,
                        i.device_manufacturer.clone(),
                        i.device_type.clone(),
                        i.device_brand.clone(),
                        i.device_os.clone(),
                        i.device_display.clone(),
                        i.browser_name.clone(),
                        i.browser_version.clone(),
                        i.issuer_id.clone(),
                        i.scheme_name.clone(),
                        i.exemption_requested,
                        i.exemption_accepted,
                        TimeRange {
                            start_time: match (granularity, i.start_bucket) {
                                (Some(g), Some(st)) => g.clip_to_start(st)?,
                                _ => time_range.start_time,
                            },
                            end_time: granularity.as_ref().map_or_else(
                                || Ok(time_range.end_time),
                                |g| i.end_bucket.map(|et| g.clip_to_end(et)).transpose(),
                            )?,
                        },
                    ),
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
