use api_models::analytics::{sdk_events::SdkEventDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};

pub trait SdkEventFilterAnalytics: LoadRow<SdkEventFilter> {}

/// Asynchronously retrieves SDK event filters for a specific dimension, publishable key, and time range from the specified data source.
pub async fn get_sdk_event_filter_for_dimension<T>(
    dimension: SdkEventDimensions,
    publishable_key: &String,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<SdkEventFilter>>
where
    T: AnalyticsDataSource + SdkEventFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::SdkEvents);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_clause("merchant_id", publishable_key)
        .switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<SdkEventFilter, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct SdkEventFilter {
    pub payment_method: Option<String>,
    pub platform: Option<String>,
    pub browser_name: Option<String>,
    pub source: Option<String>,
    pub component: Option<String>,
    pub payment_experience: Option<String>,
}
