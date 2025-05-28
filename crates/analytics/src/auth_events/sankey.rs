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
    pub authentication_status: String,
    #[serde(skip)]
    pub exemption_requested: Option<bool>,
    #[serde(skip)]
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
    // First query - Get total 3DS requests
    let mut query_builder =
        QueryBuilder::<ClickhouseClient>::new(AnalyticsCollection::Authentications);

    // Base select columns
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

    // Add filters
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

    let results = query_builder
        .execute_query::<SankeyRow, _>(clickhouse_client)
        .await
        .change_context(MetricsError::QueryBuildingError)?
        .change_context(MetricsError::QueryExecutionFailure)?;

    let mut sankey_data = Vec::new();

    let total_3ds = results.iter().map(|r| r.count).sum::<i64>();

    let exemption_requested = results
        .iter()
        .filter(|r| r.exemption_requested.unwrap_or(false))
        .map(|r| r.count)
        .sum::<i64>();

    let exemption_accepted = results
        .iter()
        .filter(|r| r.exemption_accepted.unwrap_or(false))
        .map(|r| r.count)
        .sum::<i64>();

    let completed_3ds = results
        .iter()
        .filter(|r| r.authentication_status == "success" || r.authentication_status == "failed")
        .map(|r| r.count)
        .sum::<i64>();

    let auth_success = results
        .iter()
        .filter(|r| r.authentication_status == "success")
        .map(|r| r.count)
        .sum::<i64>();

    let auth_failure = results
        .iter()
        .filter(|r| r.authentication_status == "failed")
        .map(|r| r.count)
        .sum::<i64>();

    sankey_data.push(SankeyRow {
        count: total_3ds,
        authentication_status: "Total 3DS Payment Request".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: exemption_requested,
        authentication_status: "Exemption Requested".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: total_3ds - exemption_requested,
        authentication_status: "Exemption not Requested".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: exemption_accepted,
        authentication_status: "Exemption Accepted".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: exemption_requested - exemption_accepted,
        authentication_status: "Exemption not Accepted".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: completed_3ds,
        authentication_status: "3DS Completed".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: total_3ds - completed_3ds,
        authentication_status: "3DS not Completed".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: auth_success,
        authentication_status: "Authentication Success".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    sankey_data.push(SankeyRow {
        count: auth_failure,
        authentication_status: "Authentication Failure".to_string(),
        exemption_requested: None,
        exemption_accepted: None,
    });

    Ok(sankey_data)
}
