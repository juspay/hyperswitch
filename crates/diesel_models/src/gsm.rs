//! Gateway status mapping

use common_utils::{
    custom_serde,
    events::{ApiEventMetric, ApiEventsType},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::schema::gateway_status_map;

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    router_derive::DebugAsDisplay,
    Identifiable,
    Queryable,
    serde::Serialize,
)]
#[diesel(table_name = gateway_status_map, primary_key(connector, flow, sub_flow, code, message))]
pub struct GatewayStatusMap {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: String,
    pub router_error: Option<String>,
    pub decision: String,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "custom_serde::iso8601")]
    pub last_modified: PrimitiveDateTime,
    pub step_up_possible: bool,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Insertable)]
#[diesel(table_name = gateway_status_map)]
pub struct GatewayStatusMappingNew {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: String,
    pub router_error: Option<String>,
    pub decision: String,
    pub step_up_possible: bool,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    AsChangeset,
    router_derive::DebugAsDisplay,
    Default,
    serde::Deserialize,
)]
#[diesel(table_name = gateway_status_map)]
pub struct GatewayStatusMapperUpdateInternal {
    pub connector: Option<String>,
    pub flow: Option<String>,
    pub sub_flow: Option<String>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
    pub router_error: Option<Option<String>>,
    pub decision: Option<String>,
    pub step_up_possible: Option<bool>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

#[derive(Debug)]
pub struct GatewayStatusMappingUpdate {
    pub status: Option<String>,
    pub router_error: Option<Option<String>>,
    pub decision: Option<String>,
    pub step_up_possible: Option<bool>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

impl From<GatewayStatusMappingUpdate> for GatewayStatusMapperUpdateInternal {
    fn from(value: GatewayStatusMappingUpdate) -> Self {
        let GatewayStatusMappingUpdate {
            decision,
            status,
            router_error,
            step_up_possible,
            unified_code,
            unified_message,
        } = value;
        Self {
            status,
            router_error,
            decision,
            step_up_possible,
            unified_code,
            unified_message,
            ..Default::default()
        }
    }
}

impl ApiEventMetric for GatewayStatusMap {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}
