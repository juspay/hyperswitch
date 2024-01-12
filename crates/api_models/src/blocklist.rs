use common_enums::enums;
use common_utils::events::ApiEventMetric;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum BlocklistRequest {
    CardBin(String),
    Fingerprint(String),
    ExtendedCardBin(String),
}

pub type AddToBlocklistRequest = BlocklistRequest;
pub type DeleteFromBlocklistRequest = BlocklistRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistResponse {
    pub fingerprint_id: String,
    pub data_kind: enums::BlocklistDataKind,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

pub type AddToBlocklistResponse = BlocklistResponse;
pub type DeleteFromBlocklistResponse = BlocklistResponse;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListBlocklistQuery {
    pub data_kind: enums::BlocklistDataKind,
    #[serde(default = "default_list_limit")]
    pub limit: u16,
    #[serde(default)]
    pub offset: u16,
}

fn default_list_limit() -> u16 {
    10
}

impl ApiEventMetric for BlocklistRequest {}
impl ApiEventMetric for BlocklistResponse {}
impl ApiEventMetric for ListBlocklistQuery {}
