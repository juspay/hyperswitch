use api_models::analytics::{outgoing_webhook_event::OutgoingWebhookLogsRequest, Granularity};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};
pub trait OutgoingWebhookLogsFilterAnalytics: LoadRow<OutgoingWebhookLogsResult> {}

pub async fn get_outgoing_webhook_event<T>(
    merchant_id: &String,
    query_param: OutgoingWebhookLogsRequest,
    pool: &T,
) -> FiltersResult<Vec<OutgoingWebhookLogsResult>>
where
    T: AnalyticsDataSource + OutgoingWebhookLogsFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> =
        QueryBuilder::new(AnalyticsCollection::OutgoingWebhookEvent);
    query_builder.add_select_column("*").switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;
    query_builder
        .add_filter_clause("payment_id", query_param.payment_id)
        .switch()?;

    if let Some(event_id) = query_param.event_id {
        query_builder
            .add_filter_clause("event_id", &event_id)
            .switch()?;
    }
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
    if let Some(mandate_id) = query_param.mandate_id {
        query_builder
            .add_filter_clause("mandate_id", &mandate_id)
            .switch()?;
    }
    if let Some(payment_method_id) = query_param.payment_method_id {
        query_builder
            .add_filter_clause("payment_method_id", &payment_method_id)
            .switch()?;
    }
    if let Some(attempt_id) = query_param.attempt_id {
        query_builder
            .add_filter_clause("attempt_id", &attempt_id)
            .switch()?;
    }
    //TODO!: update the execute_query function to return reports instead of plain errors...
    query_builder
        .execute_query::<OutgoingWebhookLogsResult, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OutgoingWebhookLogsResult {
    pub merchant_id: String,
    pub event_id: String,
    pub event_type: String,
    pub outgoing_webhook_event_type: String,
    pub payment_id: String,
    pub refund_id: Option<String>,
    pub attempt_id: Option<String>,
    pub dispute_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub mandate_id: Option<String>,
    pub content: Option<String>,
    pub is_error: bool,
    pub error: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}
