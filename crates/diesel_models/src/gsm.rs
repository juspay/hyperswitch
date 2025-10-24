//! Gateway status mapping
use common_enums::ErrorCategory;
use common_utils::{
    custom_serde,
    events::{ApiEventMetric, ApiEventsType},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::gateway_status_map;

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    router_derive::DebugAsDisplay,
    Identifiable,
    Queryable,
    Selectable,
    serde::Serialize,
)]
#[diesel(table_name = gateway_status_map, primary_key(connector, flow, sub_flow, code, message), check_for_backend(diesel::pg::Pg))]
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
    pub error_category: Option<ErrorCategory>,
    pub clear_pan_possible: bool,
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
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
    pub error_category: Option<ErrorCategory>,
    pub clear_pan_possible: bool,
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize,
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
    pub error_category: Option<ErrorCategory>,
    pub last_modified: PrimitiveDateTime,
    pub clear_pan_possible: Option<bool>,
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
}

#[derive(Debug)]
pub struct GatewayStatusMappingUpdate {
    pub status: Option<String>,
    pub router_error: Option<Option<String>>,
    pub decision: Option<String>,
    pub step_up_possible: Option<bool>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub error_category: Option<ErrorCategory>,
    pub clear_pan_possible: Option<bool>,
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
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
            error_category,
            clear_pan_possible,
            feature_data,
            feature,
        } = value;
        Self {
            status,
            router_error,
            decision,
            step_up_possible,
            unified_code,
            unified_message,
            error_category,
            last_modified: common_utils::date_time::now(),
            connector: None,
            flow: None,
            sub_flow: None,
            code: None,
            message: None,
            clear_pan_possible,
            feature_data,
            feature,
        }
    }
}

impl ApiEventMetric for GatewayStatusMap {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}
