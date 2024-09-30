use common_enums::enums;
use common_utils::{errors::{ErrorSwitch, ParsingError}, types::{authentication::AuthInfo, TimeRange}};
use error_stack::ResultExt;
use sqlx::query_builder;
use time::PrimitiveDateTime;

use crate::{
    clickhouse::ClickhouseClient,
    query::{Aggregate, QueryBuilder, QueryFilter},
    types::{AnalyticsCollection, DBEnumWrapper, MetricsError, MetricsResult},
};

#[derive(Debug, PartialEq, Eq, serde::Deserialize, Hash)]
pub struct PaymentIntentMetricRow {
    pub profile_id: Option<String>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<enums::AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    pub include_smart_retries: Option<bool>,
    pub first_attempt: Option<i64>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

#[derive(Debug, serde::Deserialize, strum::AsRefStr, strum::EnumString, strum::Display)]
pub enum SessionizerRefundStatus {
    Refunded,
    NotRefunded,
    PartiallyRefunded,
}

#[derive(Debug, serde::Deserialize)]
pub struct SannkeyRow {
    pub status: DBEnumWrapper<enums::IntentStatus>,
    pub refunds_status: DBEnumWrapper<SessionizerRefundStatus>,
    pub attempt_count: i64,
    pub count: i64
}

impl TryInto<SannkeyRow> for serde_json::Value {
    type Error = error_stack::Report<ParsingError>;

    fn try_into(self) -> Result<SannkeyRow, Self::Error> {
        serde_json::from_value(self).change_context(ParsingError::StructParseFailure(
            "Failed to parse Sannkey in clickhouse results",
        ))
    }
}

pub async fn get_sankey_data(
    clickhouse_client: &ClickhouseClient,
    auth: &AuthInfo,
    time_range: &TimeRange
) -> MetricsResult<Vec<SannkeyRow>> {
    let mut query_builder = QueryBuilder::<ClickhouseClient>::new(AnalyticsCollection::PaymentIntent);
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
        .add_select_column("attempt_count")
        .attach_printable("Error adding select clause")
        .change_context(MetricsError::QueryBuildingError)?;
    auth.set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    time_range.set_filter_clause(&mut query_builder)
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder.add_group_by_clause("status")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;
    query_builder.add_group_by_clause("refunds_status")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;
    query_builder.add_group_by_clause("attempt_count")
        .attach_printable("Error adding group by clause")
        .change_context(MetricsError::QueryBuildingError)?;

    query_builder
        .execute_query::<SannkeyRow, _>(clickhouse_client)
        .await
        .change_context(MetricsError::QueryBuildingError)?
        .change_context(MetricsError::QueryExecutionFailure)?
        .into_iter()
        .map(|i| Ok(i))
        .collect()
}
