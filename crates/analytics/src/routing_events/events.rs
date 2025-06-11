use api_models::analytics::{routing_events::RoutingEventsRequest, Granularity};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};
pub trait RoutingEventLogAnalytics: LoadRow<RoutingEventsResult> {}

pub async fn get_routing_events<T>(
    merchant_id: &common_utils::id_type::MerchantId,
    query_param: RoutingEventsRequest,
    pool: &T,
) -> FiltersResult<Vec<RoutingEventsResult>>
where
    T: AnalyticsDataSource + RoutingEventLogAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::RoutingEvents);
    query_builder.add_select_column("*").switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;

    query_builder
        .add_filter_clause("payment_id", &query_param.payment_id)
        .switch()?;

    if let Some(refund_id) = query_param.refund_id {
        query_builder
            .add_filter_clause("refund_id", &refund_id)
            .switch()?;
    }

    if let Some(dispute_id) = query_param.dispute_id {
        query_builder
            .add_filter_clause("dispute_id", &dispute_id)
            .switch()?;
    }

    query_builder
        .execute_query::<RoutingEventsResult, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RoutingEventsResult {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub payment_id: String,
    pub routable_connectors: String,
    pub payment_connector: Option<String>,
    pub request_id: Option<String>,
    pub flow: String,
    pub url: Option<String>,
    pub request: String,
    pub response: Option<String>,
    pub error: Option<String>,
    pub status_code: Option<u16>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub method: String,
    pub routing_engine: String,
    pub routing_approach: Option<String>,
}
