use api_models::analytics::{
    connector_events::{ConnectorEventsRequest, QueryType},
    Granularity,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};
pub trait ConnectorEventLogAnalytics: LoadRow<ConnectorEventsResult> {}

pub async fn get_connector_events<T>(
    merchant_id: &String,
    query_param: ConnectorEventsRequest,
    pool: &T,
) -> FiltersResult<Vec<ConnectorEventsResult>>
where
    T: AnalyticsDataSource + ConnectorEventLogAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> =
        QueryBuilder::new(AnalyticsCollection::ConnectorEvents);
    query_builder.add_select_column("*").switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;
    match query_param.query_param {
        QueryType::Payment { payment_id } => query_builder
            .add_filter_clause("payment_id", payment_id)
            .switch()?,
    }
    //TODO!: update the execute_query function to return reports instead of plain errors...
    query_builder
        .execute_query::<ConnectorEventsResult, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectorEventsResult {
    pub merchant_id: String,
    pub payment_id: String,
    pub connector_name: Option<String>,
    pub request_id: Option<String>,
    pub flow: String,
    pub request: String,
    pub response: Option<String>,
    pub error: Option<String>,
    pub status_code: u16,
    pub latency: Option<u128>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub method: Option<String>,
}
