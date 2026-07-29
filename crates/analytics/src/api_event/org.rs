use api_models::analytics::{GetUserActivityLogRequest, Granularity};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::critical_actions;
use crate::{
    query::{
        Aggregate, FilterTypes, GroupByClause, Order, QueryBuilder, QueryFilter, ToSql, Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
};

pub trait OrgUserActivityLogFilterAnalytics: LoadRow<OrgUserActivityLogRow> {}

/// Fetches the curated, org-scoped list of critical dashboard-user actions from the
/// `api_events` ClickHouse table.
///
/// Org-scoping is done by the caller resolving every `merchant_id` in the organization
/// via Postgres and passing them in here — we filter `merchant_id IN (...)` directly
/// rather than going through the generic `AuthInfo` filter, since (unlike every other
/// analytics table) `api_events` has no `organization_id` column; `AuthInfo::MerchantLevel`
/// would otherwise add a filter on a column that doesn't exist on this table.
///
/// Only metadata columns are selected; `request`/`response`/`error`/`authentication_data`
/// are intentionally never read here since they may contain secrets.
pub async fn get_org_user_activity_log<T>(
    merchant_ids: &[common_utils::id_type::MerchantId],
    req: &GetUserActivityLogRequest,
    pool: &T,
) -> FiltersResult<Vec<OrgUserActivityLogRow>>
where
    T: AnalyticsDataSource + OrgUserActivityLogFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::ApiEvents);

    query_builder.add_select_column("created_at").switch()?;
    query_builder.add_select_column("user_id").switch()?;
    query_builder.add_select_column("api_flow").switch()?;
    query_builder.add_select_column("merchant_id").switch()?;
    query_builder.add_select_column("ip_addr").switch()?;
    query_builder.add_select_column("status_code").switch()?;

    query_builder
        .add_filter_in_range_clause("merchant_id", merchant_ids)
        .attach_printable("Error adding merchant_id filter")
        .switch()?;

    req.time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_in_range_clause("api_flow", &critical_actions::critical_action_flows())
        .attach_printable("Error adding api_flow allowlist filter")
        .switch()?;

    query_builder
        .add_custom_filter_clause("user_id", "", FilterTypes::IsNotNull)
        .attach_printable("Error adding user_id IS NOT NULL filter")
        .switch()?;

    query_builder
        .add_order_by_clause("created_at", Order::Descending)
        .attach_printable("Error adding order by created_at")
        .switch()?;

    query_builder.set_limit_offset(req.limit, req.offset);

    query_builder
        .execute_query::<OrgUserActivityLogRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

/// A single critical-action row, scoped to metadata only — never the raw request/response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrgUserActivityLogRow {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub user_id: Option<String>,
    pub api_flow: String,
    pub ip_addr: Option<String>,
    pub status_code: u16,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}
