use api_models::analytics::{payment_intents::PaymentIntentDimensions, Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::{AuthenticationType, Currency, IntentStatus};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, FiltersError, FiltersResult,
        LoadRow,
    },
};

pub trait PaymentIntentFilterAnalytics: LoadRow<PaymentIntentFilterRow> {}

pub async fn get_payment_intent_filter_for_dimension<T>(
    dimension: PaymentIntentDimensions,
    merchant_id: &common_utils::id_type::MerchantId,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<PaymentIntentFilterRow>>
where
    T: AnalyticsDataSource + PaymentIntentFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::PaymentIntent);

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
        .execute_query::<PaymentIntentFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct PaymentIntentFilterRow {
    pub status: Option<DBEnumWrapper<IntentStatus>>,
    pub currency: Option<DBEnumWrapper<Currency>>,
    pub profile_id: Option<String>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
}
