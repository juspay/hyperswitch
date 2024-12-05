use common_enums::enums;
use common_utils::{
    errors::ParsingError,
    types::{authentication::AuthInfo, TimeRange},
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    clickhouse::ClickhouseClient,
    query::{Aggregate, QueryBuilder, QueryFilter},
    types::{AnalyticsCollection, DBEnumWrapper, MetricsError, MetricsResult},
};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionizerRefundStatus {
    FullRefunded,
    #[default]
    NotRefunded,
    PartialRefunded,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionizerDisputeStatus {
    DisputePresent,
    #[default]
    NotDisputed,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SankeyRow {
    pub count: i64,
    pub status: DBEnumWrapper<enums::IntentStatus>,
    #[serde(default)]
    pub refunds_status: Option<DBEnumWrapper<SessionizerRefundStatus>>,
    #[serde(default)]
    pub dispute_status: Option<DBEnumWrapper<SessionizerDisputeStatus>>,
    pub first_attempt: i64,
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
        QueryBuilder::<ClickhouseClient>::new(AnalyticsCollection::PaymentIntentSessionized);
    query_builder
        .add_select_column(Aggregate::<String>::Count {
            field: None,
            alias: Some("count"),
        })
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("status")
        .attach_printable("Error adding select clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("refunds_status")
        .attach_printable("Error adding select clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("dispute_status")
        .attach_printable("Error adding select clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column("(attempt_count = 1) as first_attempt")
        .attach_printable("Error adding select clause")
        .change_context(MetricsError::QueryBuildingError)?;

    auth.set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    time_range
        .set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("status")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("refunds_status")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("dispute_status")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("first_attempt")
        .attach_printable("Error adding group by clause")
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
