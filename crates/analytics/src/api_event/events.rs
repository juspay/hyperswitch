use api_models::analytics::{
    api_event::{ApiLogsRequest, QueryType},
    Granularity,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use router_env::Flow;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};
pub trait ApiLogsFilterAnalytics: LoadRow<ApiLogsResult> {}

pub async fn get_api_event<T>(
    merchant_id: &String,
    query_param: ApiLogsRequest,
    pool: &T,
) -> FiltersResult<Vec<ApiLogsResult>>
where
    T: AnalyticsDataSource + ApiLogsFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::ApiEvents);
    query_builder.add_select_column("*").switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;
    match query_param.query_param {
        QueryType::Payment { payment_id } => {
            query_builder
                .add_filter_clause("payment_id", payment_id)
                .switch()?;
            query_builder
                .add_filter_in_range_clause(
                    "api_flow",
                    &[
                        Flow::PaymentsCancel,
                        Flow::PaymentsCapture,
                        Flow::PaymentsConfirm,
                        Flow::PaymentsCreate,
                        Flow::PaymentsStart,
                        Flow::PaymentsUpdate,
                    ],
                )
                .switch()?;
        }
        QueryType::Refund {
            payment_id,
            refund_id,
        } => {
            query_builder
                .add_filter_clause("payment_id", payment_id)
                .switch()?;
            query_builder
                .add_filter_clause("refund_id", refund_id)
                .switch()?;
            query_builder
                .add_filter_in_range_clause("api_flow", &[Flow::RefundsCreate, Flow::RefundsUpdate])
                .switch()?;
        }
        QueryType::Dispute {
            payment_id,
            dispute_id,
        } => {
            query_builder
                .add_filter_clause("payment_id", payment_id)
                .switch()?;
            query_builder
                .add_filter_clause("dispute_id", dispute_id)
                .switch()?;
            query_builder
                .add_filter_in_range_clause(
                    "api_flow",
                    &[
                        Flow::DisputesEvidenceSubmit,
                        Flow::AttachDisputeEvidence,
                        Flow::RetrieveDisputeEvidence,
                    ],
                )
                .switch()?;
        }
    }
    //TODO!: update the execute_query function to return reports instead of plain errors...
    query_builder
        .execute_query::<ApiLogsResult, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiLogsResult {
    pub merchant_id: String,
    pub payment_id: Option<String>,
    pub refund_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub customer_id: Option<String>,
    pub user_id: Option<String>,
    pub connector: Option<String>,
    pub request_id: Option<String>,
    pub flow_type: String,
    pub api_flow: String,
    pub api_auth_type: Option<String>,
    pub request: String,
    pub response: Option<String>,
    pub error: Option<String>,
    pub authentication_data: Option<String>,
    pub status_code: u16,
    pub latency: Option<u128>,
    pub user_agent: Option<String>,
    pub hs_latency: Option<u128>,
    pub ip_addr: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub http_method: Option<String>,
    pub url_path: Option<String>,
}
