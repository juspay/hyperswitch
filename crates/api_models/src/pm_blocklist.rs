use common_utils::events::ApiEventMetric;

/// The request body for blacklisting pm
// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct BlocklistPmRequest {
//     pub blocklist_pm: BlocklistType,
// }

/// The request body for unblocking pm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnblockPmRequest {
    pub data: Vec<String>,
}

/// Response to be sent for the pm/block api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BlocklistPmResponse {
    pub blocked: BlocklistType,
}

/// Response to be sent for the list blocked PaymentMethods api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ListBlockedPmResponse {
    pub blocked_fingerprints: Vec<String>,
    pub blocked_card_bins: Vec<(String, Option<String>)>,
    pub blocked_extended_bins: Vec<(String, Option<String>)>,
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
    CardBin(Vec<String>),
    Fingerprint(Vec<String>),
    ExtendedBin(Vec<String>),
}

impl ApiEventMetric for BlocklistPmResponse {}
impl ApiEventMetric for BlocklistType {}
impl ApiEventMetric for UnblockPmResponse {}
impl ApiEventMetric for UnblockPmRequest {}
impl ApiEventMetric for ListBlockedPmResponse {}
