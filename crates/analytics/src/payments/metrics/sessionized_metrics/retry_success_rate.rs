use std::collections::HashSet;

use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::PaymentMetricRow;
use crate::{
    enums::AuthInfo,
    query::{
        Aggregate, FilterTypes, GroupByClause, Order, QueryBuilder, QueryFilter, SeriesBucket,
        ToSql, Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

/// Retry success rate grouped by error type (standardised_code).
///
/// Correlates first-attempt failures with retry outcomes:
/// - Outer query: counts first-attempt failures grouped by
///   `standardised_code` and `error_category`.
/// - Inner subquery: counts successful retry attempts
///   (`first_attempt = false`, `status = 'charged'`) across all error
///   types to provide a retry-success baseline for normalization.
///
/// The accumulator uses `count` (per-error-type first-attempt failures)
/// and `total` (overall successful retries) to compute the ratio.
#[derive(Default)]
pub(crate) struct RetrySuccessRateByErrorType;

#[async_trait::async_trait]
impl<T> super::PaymentMetric<T> for RetrySuccessRateByErrorType
where
    T: AnalyticsDataSource + super::PaymentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        // Inner subquery: total successful retry attempts (for normalization).
        // Counts attempts where first_attempt = false AND status = 'charged'.
        let mut inner_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentSessionized);
        inner_query_builder
            .add_select_column("sum(sign_flag)")
            .switch()?;

        inner_query_builder
            .add_bool_filter_clause("first_attempt", false)
            .switch()?;

        inner_query_builder
            .add_filter_clause(
                PaymentDimensions::PaymentStatus,
                storage_enums::AttemptStatus::Charged,
            )
            .switch()?;

        time_range
            .set_filter_clause(&mut inner_query_builder)
            .attach_printable("Error filtering time range for inner query")
            .switch()?;

        auth.set_filter_clause(&mut inner_query_builder)
            .switch()?;

        let inner_query_string = inner_query_builder
            .build_query()
            .attach_printable("Error building inner query")
            .change_context(MetricsError::QueryBuildingError)?;

        // Outer query: first-attempt failures grouped by standardised_code
        let mut outer_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentSessionized);

        for dim in dimensions.iter() {
            outer_query_builder.add_select_column(dim).switch()?;
        }

        outer_query_builder
            .add_select_column("sum(sign_flag) AS count")
            .switch()?;

        outer_query_builder
            .add_select_column(format!("({inner_query_string}) AS total"))
            .switch()?;

        outer_query_builder
            .add_select_column("first_attempt")
            .switch()?;

        outer_query_builder
            .add_select_column(PaymentDimensions::ErrorCategory)
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;

        filters
            .set_filter_clause(&mut outer_query_builder)
            .switch()?;

        auth.set_filter_clause(&mut outer_query_builder).switch()?;

        time_range
            .set_filter_clause(&mut outer_query_builder)
            .attach_printable("Error filtering time range for outer query")
            .switch()?;

        // Only first-attempt failures
        outer_query_builder
            .add_filter_clause(
                PaymentDimensions::PaymentStatus,
                storage_enums::AttemptStatus::Failure,
            )
            .switch()?;

        outer_query_builder
            .add_bool_filter_clause("first_attempt", true)
            .switch()?;

        outer_query_builder
            .add_custom_filter_clause(
                PaymentDimensions::StandardisedCode,
                "NULL",
                FilterTypes::IsNotNull,
            )
            .switch()?;

        for dim in dimensions.iter() {
            outer_query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

        outer_query_builder
            .add_group_by_clause(PaymentDimensions::StandardisedCode)
            .attach_printable("Error grouping by standardised_code")
            .switch()?;

        outer_query_builder
            .add_group_by_clause("first_attempt")
            .attach_printable("Error grouping by first_attempt")
            .switch()?;

        outer_query_builder
            .add_group_by_clause(PaymentDimensions::ErrorCategory)
            .attach_printable("Error grouping by error_category")
            .switch()?;

        if let Some(granularity) = granularity {
            granularity
                .set_group_by_clause(&mut outer_query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        outer_query_builder
            .add_order_by_clause("count", Order::Descending)
            .attach_printable("Error adding order by clause")
            .switch()?;

        let filtered_dimensions: Vec<&PaymentDimensions> = dimensions
            .iter()
            .filter(|&&dim| dim != PaymentDimensions::StandardisedCode)
            .collect();

        for dim in &filtered_dimensions {
            outer_query_builder
                .add_order_by_clause(*dim, Order::Ascending)
                .attach_printable("Error adding order by clause")
                .switch()?;
        }

        outer_query_builder
            .execute_query::<PaymentMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        None,
                        i.connector.clone(),
                        i.authentication_type.as_ref().map(|i| i.0),
                        i.payment_method.clone(),
                        i.payment_method_type.clone(),
                        i.client_source.clone(),
                        i.client_version.clone(),
                        i.profile_id.clone(),
                        i.card_network.clone(),
                        i.merchant_id.clone(),
                        i.card_last_4.clone(),
                        i.card_issuer.clone(),
                        i.error_reason.clone(),
                        i.routing_approach.as_ref().map(|i| i.0.clone()),
                        i.signature_network.clone(),
                        i.is_issuer_regulated,
                        i.is_debit_routed,
                        i.standardised_code.clone(),
                        i.error_category.clone(),
                        i.unified_code.clone(),
                        i.unified_message.clone(),
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
                HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}

/// Retry success rate grouped by connector.
///
/// Correlates retry attempts with their outcomes per connector:
/// - Outer query: counts retry attempts (`first_attempt = false`)
///   grouped by connector and status, so the accumulator can
///   distinguish successful retries from failed retries per connector.
/// - Inner subquery: counts total retry attempts across all connectors
///   for normalization.
#[derive(Default)]
pub(crate) struct RetrySuccessRateByConnector;

#[async_trait::async_trait]
impl<T> super::PaymentMetric<T> for RetrySuccessRateByConnector
where
    T: AnalyticsDataSource + super::PaymentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        // Inner subquery: count SUCCESSFUL retry attempts (for success rate calculation)
        let mut inner_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentSessionized);
        inner_query_builder
            .add_select_column("sum(sign_flag)")
            .switch()?;

        inner_query_builder
            .add_bool_filter_clause("first_attempt", false)
            .switch()?;

        inner_query_builder
            .add_filter_clause(
                PaymentDimensions::PaymentStatus,
                storage_enums::AttemptStatus::Charged,
            )
            .switch()?;

        time_range
            .set_filter_clause(&mut inner_query_builder)
            .attach_printable("Error filtering time range for inner query")
            .switch()?;

        auth.set_filter_clause(&mut inner_query_builder)
            .switch()?;

        let inner_query_string = inner_query_builder
            .build_query()
            .attach_printable("Error building inner query")
            .change_context(MetricsError::QueryBuildingError)?;

        // Outer query: ALL retry attempts grouped by connector
        let mut outer_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentSessionized);

        for dim in dimensions.iter() {
            outer_query_builder.add_select_column(dim).switch()?;
        }

        outer_query_builder
            .add_select_column(PaymentDimensions::Connector)
            .switch()?;

        outer_query_builder
            .add_select_column("sum(sign_flag) AS count")
            .switch()?;

        outer_query_builder
            .add_select_column(format!("({inner_query_string}) AS total"))
            .switch()?;

        outer_query_builder
            .add_select_column("first_attempt")
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;

        filters
            .set_filter_clause(&mut outer_query_builder)
            .switch()?;

        auth.set_filter_clause(&mut outer_query_builder).switch()?;

        time_range
            .set_filter_clause(&mut outer_query_builder)
            .attach_printable("Error filtering time range for outer query")
            .switch()?;

        // Only retry attempts (non-first attempts)
        outer_query_builder
            .add_bool_filter_clause("first_attempt", false)
            .switch()?;

        for dim in dimensions.iter() {
            outer_query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

        outer_query_builder
            .add_group_by_clause(PaymentDimensions::Connector)
            .attach_printable("Error grouping by connector")
            .switch()?;

        outer_query_builder
            .add_group_by_clause("first_attempt")
            .attach_printable("Error grouping by first_attempt")
            .switch()?;

        if let Some(granularity) = granularity {
            granularity
                .set_group_by_clause(&mut outer_query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        outer_query_builder
            .add_order_by_clause("count", Order::Descending)
            .attach_printable("Error adding order by clause")
            .switch()?;

        let filtered_dimensions: Vec<&PaymentDimensions> = dimensions
            .iter()
            .filter(|&&dim| dim != PaymentDimensions::Connector)
            .collect();

        for dim in &filtered_dimensions {
            outer_query_builder
                .add_order_by_clause(*dim, Order::Ascending)
                .attach_printable("Error adding order by clause")
                .switch()?;
        }

        outer_query_builder
            .execute_query::<PaymentMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        i.status.as_ref().map(|i| i.0),
                        i.connector.clone(),
                        i.authentication_type.as_ref().map(|i| i.0),
                        i.payment_method.clone(),
                        i.payment_method_type.clone(),
                        i.client_source.clone(),
                        i.client_version.clone(),
                        i.profile_id.clone(),
                        i.card_network.clone(),
                        i.merchant_id.clone(),
                        i.card_last_4.clone(),
                        i.card_issuer.clone(),
                        i.error_reason.clone(),
                        i.routing_approach.as_ref().map(|i| i.0.clone()),
                        i.signature_network.clone(),
                        i.is_issuer_regulated,
                        i.is_debit_routed,
                        i.standardised_code.clone(),
                        i.error_category.clone(),
                        i.unified_code.clone(),
                        i.unified_message.clone(),
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
                HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
