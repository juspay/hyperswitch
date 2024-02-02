use api_models::analytics::{
    refunds::{RefundDimensions, RefundType},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::{Currency, RefundStatus};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, FiltersError, FiltersResult,
        LoadRow,
    },
};
pub trait RefundFilterAnalytics: LoadRow<RefundFilterRow> {}

/// Asynchronously retrieves refund filters for a given dimension, merchant, and time range using the provided pool for database access.
pub async fn get_refund_filter_for_dimension<T>(
    dimension: RefundDimensions,
    merchant: &String,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<RefundFilterRow>>
where
    T: AnalyticsDataSource + RefundFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::Refund);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant)
        .switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<RefundFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}
#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct RefundFilterRow {
    pub currency: Option<DBEnumWrapper<Currency>>,
    pub refund_status: Option<DBEnumWrapper<RefundStatus>>,
    pub connector: Option<String>,
    pub refund_type: Option<DBEnumWrapper<RefundType>>,
}
