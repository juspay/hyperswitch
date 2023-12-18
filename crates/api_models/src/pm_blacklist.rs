use common_utils::events::ApiEventMetric;

/// The request body for blacklisting pm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistPmRequest {
    pub pm_to_block: Vec<String>,
}

/// The request body for unblocking pm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnblockPmRequest {
    pub data: Vec<String>,
}

/// Response to be sent for the pm/block api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BlacklistPmResponse {
    pub fingerprints_blocked: Vec<String>,
}

/// Response to be sent for the list blocked PaymentMethods api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ListBlockedPmResponse {
    pub fingerprints_blocked: Vec<String>,
}

/// Response to be sent for the pm/unblock api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UnblockPmResponse {
    pub data: Vec<String>,
}

impl ApiEventMetric for BlacklistPmRequest {}
impl ApiEventMetric for BlacklistPmResponse {}
impl ApiEventMetric for UnblockPmResponse {}
impl ApiEventMetric for UnblockPmRequest {}
impl ApiEventMetric for ListBlockedPmResponse {}
