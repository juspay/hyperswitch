use utoipa::ToSchema;

use crate::enums::Connector;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmCreateRequest {
    /// The connector through which payment has gone through
    pub connector: Connector,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: String,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    pub decision: GsmDecision,
    /// indicates if step_up retry is possible
    pub step_up_possible: bool,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmRetrieveRequest {
    /// The connector through which payment has gone through
    pub connector: Connector,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
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
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: Option<String>,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    pub decision: Option<GsmDecision>,
    /// indicates if step_up retry is possible
    pub step_up_possible: Option<bool>,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmDeleteRequest {
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct GsmDeleteResponse {
    pub gsm_rule_delete: bool,
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct GsmResponse {
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: String,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    pub decision: String,
    /// indicates if step_up retry is possible
    pub step_up_possible: bool,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
}
