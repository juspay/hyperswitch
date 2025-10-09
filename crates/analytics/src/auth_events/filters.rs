use api_models::analytics::{auth_events::AuthEventDimensions, Granularity, TimeRange};
use common_enums::{Currency, DecoupledAuthenticationType};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::{AuthenticationConnectors, AuthenticationStatus, TransactionStatus};
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

pub trait AuthEventFilterAnalytics: LoadRow<AuthEventFilterRow> {}

pub async fn get_auth_events_filter_for_dimension<T>(
    dimension: AuthEventDimensions,
    auth: &AuthInfo,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<AuthEventFilterRow>>
where
    T: AnalyticsDataSource + AuthEventFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> =
        QueryBuilder::new(AnalyticsCollection::Authentications);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder.set_distinct();

    auth.set_filter_clause(&mut query_builder).switch()?;

    query_builder
        .execute_query::<AuthEventFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct AuthEventFilterRow {
    pub authentication_status: Option<DBEnumWrapper<AuthenticationStatus>>,
    pub trans_status: Option<DBEnumWrapper<TransactionStatus>>,
    pub authentication_type: Option<DBEnumWrapper<DecoupledAuthenticationType>>,
    pub error_message: Option<String>,
    pub authentication_connector: Option<DBEnumWrapper<AuthenticationConnectors>>,
    pub message_version: Option<String>,
    pub acs_reference_number: Option<String>,
    pub platform: Option<String>,
    pub mcc: Option<String>,
    pub currency: Option<DBEnumWrapper<Currency>>,
    pub merchant_country: Option<String>,
    pub billing_country: Option<String>,
    pub shipping_country: Option<String>,
    pub issuer_country: Option<String>,
    pub earliest_supported_version: Option<String>,
    pub latest_supported_version: Option<String>,
    pub whitelist_decision: Option<bool>,
    pub device_manufacturer: Option<String>,
    pub device_type: Option<String>,
    pub device_brand: Option<String>,
    pub device_os: Option<String>,
    pub device_display: Option<String>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub issuer_id: Option<String>,
    pub scheme_name: Option<String>,
    pub exemption_requested: Option<bool>,
    pub exemption_accepted: Option<bool>,
}
