use common_enums::enums;
use common_utils::events::ApiEventMetric;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum BlocklistRequest {
    CardBin(String),
    Fingerprint(String),
    ExtendedCardBin(String),
}

pub type AddToBlocklistRequest = BlocklistRequest;
pub type DeleteFromBlocklistRequest = BlocklistRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistResponse {
    pub fingerprint_id: String,
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

pub type AddToBlocklistResponse = BlocklistResponse;
pub type DeleteFromBlocklistResponse = BlocklistResponse;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBlocklistQuery {
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    #[serde(default = "default_list_limit")]
    pub limit: u16,
    #[serde(default)]
    pub offset: u16,
}

/// This method returns the default limit for a list, which is set to 10.
fn default_list_limit() -> u16 {
    10
}

impl ApiEventMetric for BlocklistRequest {}
impl ApiEventMetric for BlocklistResponse {}
impl ApiEventMetric for ListBlocklistQuery {}
