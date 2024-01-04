use common_utils::events::ApiEventMetric;

/// The request body for blacklisting pm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistPmRequest {
    pub blocklist_pm: BlocklistType,
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
    pub blocked: BlocklistType,
}

/// Response to be sent for the list blocked PaymentMethods api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ListBlockedPmResponse {
    pub blocked_fingerprints: Vec<String>,
    pub blocked_cardbins: Vec<(String, Option<String>)>,
    pub blocked_extended_cardbins: Vec<(String, Option<String>)>,
}

/// Response to be sent for the pm/unblock api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UnblockPmResponse {
    pub unblocked_pm: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum BlocklistType {
    Cardbin(Vec<String>),
    Fingerprint(Vec<String>),
    ExtendedCardbin(Vec<String>),
}

impl ApiEventMetric for BlacklistPmRequest {}
impl ApiEventMetric for BlacklistPmResponse {}
impl ApiEventMetric for UnblockPmResponse {}
impl ApiEventMetric for UnblockPmRequest {}
impl ApiEventMetric for ListBlockedPmResponse {}
