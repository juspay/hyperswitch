use crate::enums;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GsmCreateRequest {
    pub connector: enums::Connector,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: String,
    pub router_error: Option<String>,
    pub decision: GsmDecision,
    pub step_up_possible: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GsmRetrieveRequest {
    pub connector: enums::Connector,
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
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GsmDecision {
    Retry,
    Requeue,
    #[default]
    DoDefault,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
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
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GsmDeleteRequest {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GsmDeleteResponse {
    pub gsm_rule_delete: bool,
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
}
