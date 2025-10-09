use common_enums::AuthenticationStatus;
use common_utils::{
    errors::ParsingError,
    types::{authentication::AuthInfo, TimeRange},
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    clickhouse::ClickhouseClient,
    query::{Aggregate, QueryBuilder, QueryFilter},
    types::{AnalyticsCollection, MetricsError, MetricsResult},
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SankeyRow {
    pub count: i64,
    pub authentication_status: Option<AuthenticationStatus>,
    pub exemption_requested: Option<bool>,
    pub exemption_accepted: Option<bool>,
}

impl TryInto<SankeyRow> for serde_json::Value {
    type Error = error_stack::Report<ParsingError>;

    fn try_into(self) -> Result<SankeyRow, Self::Error> {
        logger::debug!("Parsing SankeyRow from {:?}", self);
        serde_json::from_value(self).change_context(ParsingError::StructParseFailure(
            "Failed to parse Sankey in clickhouse results",
        ))
    }
}

pub async fn get_sankey_data(
    clickhouse_client: &ClickhouseClient,
    auth: &AuthInfo,
    time_range: &TimeRange,
) -> MetricsResult<Vec<SankeyRow>> {
    let mut query_builder =
        QueryBuilder::<ClickhouseClient>::new(AnalyticsCollection::Authentications);

    query_builder
        .add_select_column(Aggregate::<String>::Count {
            field: None,
            alias: Some("count"),
        })
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("exemption_requested")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("exemption_accepted")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("authentication_status")
        .change_context(MetricsError::QueryBuildingError)?;

    auth.set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    time_range
        .set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("exemption_requested")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("exemption_accepted")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("authentication_status")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .execute_query::<SankeyRow, _>(clickhouse_client)
        .await
        .change_context(MetricsError::QueryBuildingError)?
        .change_context(MetricsError::QueryExecutionFailure)?
        .into_iter()
        .map(Ok)
        .collect()
}
