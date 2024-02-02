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

/// Asynchronously retrieves payment filters for a given dimension, merchant, and time range using the provided data source pool.
pub async fn get_payment_filter_for_dimension<T>(
    dimension: PaymentDimensions,
    merchant: &String,
    time_range: &TimeRange,
    pool: &T,
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
        .add_filter_clause("merchant_id", merchant)
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
}
