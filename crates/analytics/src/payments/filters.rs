use api_models::analytics::{payments::PaymentDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::{AttemptStatus, AuthenticationType, Currency};
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

pub trait PaymentFilterAnalytics: LoadRow<PaymentFilterRow> {}

pub async fn get_payment_filter_for_dimension<T>(
    dimension: PaymentDimensions,
    auth: &AuthInfo,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<PaymentFilterRow>>
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

    auth.set_filter_clause(&mut query_builder).switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<PaymentFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct PaymentFilterRow {
    pub currency: Option<DBEnumWrapper<Currency>>,
    pub status: Option<DBEnumWrapper<AttemptStatus>>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub profile_id: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    pub first_attempt: Option<bool>,
}
