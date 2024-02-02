use api_models::analytics::{api_event::ApiEventDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};

pub trait ApiEventFilterAnalytics: LoadRow<ApiEventFilter> {}

/// Asynchronously retrieves a list of API event filters for a specific dimension, merchant, and time range from the given data source pool.
pub async fn get_api_event_filter_for_dimension<T>(
    dimension: ApiEventDimensions,
    merchant_id: &String,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<ApiEventFilter>>
where
    T: AnalyticsDataSource + ApiEventFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::ApiEvents);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<ApiEventFilter, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct ApiEventFilter {
    pub status_code: Option<i32>,
    pub flow_type: Option<String>,
    pub api_flow: Option<String>,
}
