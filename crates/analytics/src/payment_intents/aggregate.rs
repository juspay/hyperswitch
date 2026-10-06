use std::collections::HashMap;

use common_enums::IntentStatus;
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct IntentStatusCountRow {
    pub status: DBEnumWrapper<IntentStatus>,
    pub count: i64,
}

impl TryInto<IntentStatusCountRow> for serde_json::Value {
    type Error = error_stack::Report<ParsingError>;

    fn try_into(self) -> Result<IntentStatusCountRow, Self::Error> {
        logger::debug!("Parsing IntentStatusCountRow from {:?}", self);
        serde_json::from_value(self).change_context(ParsingError::StructParseFailure(
            "Failed to parse IntentStatusCount in clickhouse results",
        ))
    }
}

pub async fn get_intent_status_with_count(
    clickhouse_client: &ClickhouseClient,
    auth: &AuthInfo,
    time_range: &TimeRange,
) -> MetricsResult<HashMap<IntentStatus, i64>> {
    let mut query_builder =
        QueryBuilder::<ClickhouseClient>::new(AnalyticsCollection::PaymentIntent);

    query_builder
        .add_select_column("status")
        .attach_printable("Error adding select status")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_select_column(Aggregate::<String>::Sum {
            field: "sign_flag".to_string(),
            alias: Some("count"),
        })
        .change_context(MetricsError::QueryBuildingError)?;

    auth.set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    time_range
        .set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .add_group_by_clause("status")
        .attach_printable("Error adding group by status")
        .change_context(MetricsError::QueryBuildingError)?;

    let rows: Vec<IntentStatusCountRow> = query_builder
        .execute_query::<IntentStatusCountRow, _>(clickhouse_client)
        .await
        .change_context(MetricsError::QueryBuildingError)?
        .change_context(MetricsError::QueryExecutionFailure)?;

    let mut status_map: HashMap<IntentStatus, i64> = rows
        .into_iter()
        .map(|row| (row.status.0, row.count))
        .collect();

    for status in <IntentStatus as strum::IntoEnumIterator>::iter() {
        status_map.entry(status).or_insert(0);
    }

    Ok(status_map)
}
