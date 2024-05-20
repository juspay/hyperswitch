use api_models::analytics::{payments::PaymentDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::{AttemptStatus, AuthenticationType, Currency};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, FiltersError, FiltersResult,
        LoadRow,
    },
};

pub trait PaymentFilterAnalytics: LoadRow<FilterRow> {}

pub async fn get_payment_filter_for_dimension<T>(
    dimension: PaymentDimensions,
    time_range: &TimeRange,
    pool: &T,
    merchant_ids: &[String],
) -> FiltersResult<Vec<FilterRow>>
where
    T: AnalyticsDataSource + PaymentFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::Payment);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_in_range_clause("merchant_id", merchant_ids)
        .switch()?;
    query_builder.set_distinct();
    query_builder
        .execute_query::<FilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct FilterRow {
    pub currency: Option<DBEnumWrapper<Currency>>,
    pub status: Option<DBEnumWrapper<AttemptStatus>>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub merchant_id: Option<String>,
}
