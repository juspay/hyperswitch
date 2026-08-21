use api_models::analytics::{Granularity, TimeRange};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing, Flow};
use time::PrimitiveDateTime;

use crate::{
    errors::AnalyticsResult,
    query::{Aggregate, GroupByClause, Order, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, FiltersError, FiltersResult, LoadRow},
    AnalyticsProvider,
};

/// Curated allowlist of critical dashboard actions surfaced in the org activity log
pub const ORG_ACTIVITY_LOG_FLOWS: &[Flow] = &[
    Flow::MerchantConnectorsCreate,
    Flow::MerchantConnectorsUpdate,
    Flow::MerchantConnectorsDelete,
    Flow::MerchantsAccountUpdate,
    Flow::ProfileCreate,
    Flow::ProfileUpdate,
    Flow::ProfileDelete,
    Flow::RoutingCreateConfig,
    Flow::RoutingLinkConfig,
    Flow::RoutingUnlinkConfig,
    Flow::ApiKeyCreate,
    Flow::ApiKeyUpdate,
    Flow::ApiKeyRevoke,
    Flow::InviteMultipleUser,
    Flow::UpdateUserRole,
    Flow::DeleteUserRole,
    Flow::CreateRole,
    Flow::CreateRoleV2,
    Flow::UpdateRole,
    Flow::DecisionManagerUpsertConfig,
    Flow::DecisionManagerDeleteConfig,
];

pub trait OrgActivityLogAnalytics:
    LoadRow<OrgActivityLogRow> + LoadRow<OrgActivityLogCountRow>
{
}

pub async fn get_org_activity_logs<T>(
    user_ids: &[String],
    merchant_ids: &[common_utils::id_type::MerchantId],
    api_flows: &[Flow],
    time_range: &TimeRange,
    limit: u64,
    offset: u64,
    pool: &T,
) -> FiltersResult<Vec<OrgActivityLogRow>>
where
    T: AnalyticsDataSource + OrgActivityLogAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> =
        QueryBuilder::new(AnalyticsCollection::ApiEventsAnalytics);
    for column in [
        "merchant_id",
        "auth_user_id",
        "flow_type",
        "api_flow",
        "status_code",
        "http_method",
        "url_path",
        "created_at",
    ] {
        query_builder.add_select_column(column).switch()?;
    }

    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;
    query_builder
        .add_filter_in_range_clause("auth_user_id", user_ids)
        .switch()?;
    query_builder
        .add_filter_in_range_clause("merchant_id", merchant_ids)
        .switch()?;
    query_builder
        .add_filter_in_range_clause("api_flow", api_flows)
        .switch()?;

    query_builder
        .add_order_by_clause("created_at", Order::Descending)
        .switch()?;
    query_builder.set_limit_and_offset(limit, offset);

    query_builder
        .execute_query::<OrgActivityLogRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

pub async fn get_org_activity_logs_count<T>(
    user_ids: &[String],
    merchant_ids: &[common_utils::id_type::MerchantId],
    api_flows: &[Flow],
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<OrgActivityLogCountRow>>
where
    T: AnalyticsDataSource + OrgActivityLogAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> =
        QueryBuilder::new(AnalyticsCollection::ApiEventsAnalytics);
    query_builder
        .add_select_column(Aggregate::Count {
            field: None,
            alias: Some("count"),
        })
        .switch()?;

    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;
    query_builder
        .add_filter_in_range_clause("auth_user_id", user_ids)
        .switch()?;
    query_builder
        .add_filter_in_range_clause("merchant_id", merchant_ids)
        .switch()?;
    query_builder
        .add_filter_in_range_clause("api_flow", api_flows)
        .switch()?;

    query_builder
        .execute_query::<OrgActivityLogCountRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[instrument(skip_all)]
pub async fn org_activity_log_core(
    pool: &AnalyticsProvider,
    user_ids: &[String],
    merchant_ids: &[common_utils::id_type::MerchantId],
    api_flows: &[Flow],
    time_range: &TimeRange,
    limit: u64,
    offset: u64,
) -> AnalyticsResult<(Vec<OrgActivityLogRow>, u64)> {
    let rows = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Org activity logs not implemented for SQLX",
        ))
        .attach_printable("SQL Analytics is not implemented for org activity logs"),
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
            get_org_activity_logs(
                user_ids,
                merchant_ids,
                api_flows,
                time_range,
                limit,
                offset,
                ckh_pool,
            )
            .await
        }
    }
    .switch()?;

    let total_count = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Org activity logs not implemented for SQLX",
        ))
        .attach_printable("SQL Analytics is not implemented for org activity logs"),
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
            get_org_activity_logs_count(user_ids, merchant_ids, api_flows, time_range, ckh_pool)
                .await
        }
    }
    .switch()?
    .first()
    .and_then(|row| row.count)
    .unwrap_or_default();

    Ok((rows, total_count))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgActivityLogRow {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub auth_user_id: Option<String>,
    pub flow_type: String,
    pub api_flow: String,
    pub status_code: u16,
    pub http_method: Option<String>,
    pub url_path: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgActivityLogCountRow {
    pub count: Option<u64>,
}
