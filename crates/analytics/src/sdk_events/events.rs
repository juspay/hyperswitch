use api_models::analytics::{
    sdk_events::{SdkEventNames, SdkEventsRequest},
    Granularity,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use strum::IntoEnumIterator;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, FilterTypes, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};
pub trait SdkEventsFilterAnalytics: LoadRow<SdkEventsResult> {}

pub async fn get_sdk_event<T>(
    merchant_id: &str,
    request: SdkEventsRequest,
    pool: &T,
) -> FiltersResult<Vec<SdkEventsResult>>
where
    T: AnalyticsDataSource + SdkEventsFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let static_event_list = SdkEventNames::iter()
        .map(|i| format!("'{}'", i.as_ref()))
        .collect::<Vec<String>>()
        .join(",");
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::SdkEvents);
    query_builder.add_select_column("*").switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;
    query_builder
        .add_filter_clause("payment_id", request.payment_id)
        .switch()?;
    query_builder
        .add_custom_filter_clause("event_name", static_event_list, FilterTypes::In)
        .switch()?;
    let _ = &request
        .time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    //TODO!: update the execute_query function to return reports instead of plain errors...
    query_builder
        .execute_query::<SdkEventsResult, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkEventsResult {
    pub merchant_id: String,
    pub payment_id: String,
    pub event_name: Option<String>,
    pub log_type: Option<String>,
    pub first_event: bool,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub version: Option<String>,
    pub value: Option<String>,
    pub platform: Option<String>,
    pub component: Option<String>,
    pub payment_method: Option<String>,
    pub payment_experience: Option<String>,
    pub latency: Option<u64>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at_precise: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}
