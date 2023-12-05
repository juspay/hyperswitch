use common_utils::events::ApiEventMetric;

/// The request body for blacklisting pm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistPmRequest {
    pub pm_to_block: Vec<String>,
}

/// Response to be sent for the verify/applepay api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BlacklistPmResponse {
    pub status_message: String,
    pub fingerprints_blocked: Vec<String>,
}

impl ApiEventMetric for BlacklistPmRequest {}
impl ApiEventMetric for BlacklistPmResponse {}

