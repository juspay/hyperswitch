use utoipa::ToSchema;

use crate::enums::Connector;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmCreateRequest {
    pub connector: Connector,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: String,
    pub router_error: Option<String>,
    pub decision: GsmDecision,
    pub step_up_possible: bool,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmRetrieveRequest {
    pub connector: Connector,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
}

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    strum::Display,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GsmDecision {
    Retry,
    Requeue,
    #[default]
    DoDefault,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmUpdateRequest {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: Option<String>,
    pub router_error: Option<String>,
    pub decision: Option<GsmDecision>,
    pub step_up_possible: Option<bool>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmDeleteRequest {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct GsmDeleteResponse {
    pub gsm_rule_delete: bool,
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct GsmResponse {
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
