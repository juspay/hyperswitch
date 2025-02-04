use api_models::analytics::{disputes::DisputeDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::Currency;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    enums::AuthInfo,
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, FiltersError, FiltersResult,
        LoadRow,
    },
};
pub trait DisputeFilterAnalytics: LoadRow<DisputeFilterRow> {}

pub async fn get_dispute_filter_for_dimension<T>(
    dimension: DisputeDimensions,
    auth: &AuthInfo,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<DisputeFilterRow>>
where
    T: AnalyticsDataSource + DisputeFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::Dispute);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    auth.set_filter_clause(&mut query_builder).switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<DisputeFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}
#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DisputeFilterRow {
    pub connector: Option<String>,
    pub dispute_status: Option<String>,
    pub connector_status: Option<String>,
    pub dispute_stage: Option<String>,
    pub currency: Option<DBEnumWrapper<Currency>>,
}
