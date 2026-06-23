use common_enums::enums;
use common_utils::events::ApiEventMetric;
use hyperswitch_masking::StrongSecret;
use utoipa::ToSchema;

const MAX_BATCH_LIST_LIMIT: u8 = 100;
const DEFAULT_BATCH_LIST_LIMIT: u8 = 10;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum BlocklistRequest {
    CardBin(String),
    Fingerprint(String),
    ExtendedCardBin(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GenerateFingerprintRequest {
    pub data: StrongSecret<String>,
    pub key: StrongSecret<String>,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Card {
    pub card_number: StrongSecret<String>,
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GenerateFingerprintResponsePayload {
    pub fingerprint_id: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ToggleBlocklistResponse {
    pub blocklist_guard_status: String,
}

pub type AddToBlocklistResponse = BlocklistResponse;
pub type DeleteFromBlocklistResponse = BlocklistResponse;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBlocklistResponse {
    /// The number of blocked entries in the current response
    pub count: usize,
    /// The total number of blocked entries for the given data_kind
    pub total_count: usize,
    /// The list of blocked payment method entries
    pub data: Vec<BlocklistResponse>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBlocklistQuery {
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    #[serde(default = "default_list_limit")]
    pub limit: u16,
    #[serde(default)]
    pub offset: u16,
    pub client_secret: Option<String>,
}

fn default_list_limit() -> u16 {
    10
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ToggleBlocklistQuery {
    #[schema(value_type = BlocklistDataKind)]
    pub status: bool,
}

impl ApiEventMetric for BlocklistRequest {}
impl ApiEventMetric for BlocklistResponse {}
impl ApiEventMetric for ListBlocklistResponse {}
impl ApiEventMetric for ToggleBlocklistResponse {}
impl ApiEventMetric for ListBlocklistQuery {}
impl ApiEventMetric for GenerateFingerprintRequest {}
impl ApiEventMetric for ToggleBlocklistQuery {}
impl ApiEventMetric for GenerateFingerprintResponsePayload {}
impl ApiEventMetric for Card {}

// ---- Batch Blocklist Upload types ----

/// A single validation error found in a batch-upload CSV row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistRowError {
    /// 0-based row index in the CSV (excluding header).
    pub row_index: usize,
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    pub data: String,
    pub reason: String,
}

/// Response returned on a successful `POST /blocklist/batch`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistUploadResponse {
    pub job_id: String,
    pub total_rows: u32,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
}

/// Response for `GET /blocklist/batch/{job_id}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistJobStatusResponse {
    pub job_id: String,
    pub merchant_id: String,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    pub total_rows: u32,
    pub succeeded_rows: u32,
    pub failed_rows: u32,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: time::PrimitiveDateTime,
}

/// Page size for listing batch blocklist jobs. Defaults to 10, capped at 100.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct BatchListLimit(u8);

impl BatchListLimit {
    pub fn get(&self) -> u8 {
        self.0
    }
}

impl Default for BatchListLimit {
    fn default() -> Self {
        Self(DEFAULT_BATCH_LIST_LIMIT)
    }
}

impl<'de> serde::Deserialize<'de> for BatchListLimit {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let val = u8::deserialize(deserializer)?;
        if val > MAX_BATCH_LIST_LIMIT {
            return Err(serde::de::Error::custom(format!(
                "limit must not exceed {MAX_BATCH_LIST_LIMIT}"
            )));
        }
        Ok(Self(val))
    }
}

/// Page offset for listing batch blocklist jobs. Defaults to 0.
#[derive(Debug, Clone, Default, serde::Serialize, ToSchema)]
pub struct BatchListOffset(u32);

impl BatchListOffset {
    pub fn get(&self) -> u32 {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for BatchListOffset {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(u32::deserialize(deserializer)?))
    }
}

/// Query parameters for listing batch blocklist jobs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBatchBlocklistJobsQuery {
    #[serde(default)]
    pub limit: BatchListLimit,
    #[serde(default)]
    pub offset: BatchListOffset,
}

/// Response for `GET /blocklist/batch`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBatchBlocklistJobsResponse {
    pub count: usize,
    pub total_count: usize,
    pub data: Vec<BatchBlocklistJobStatusResponse>,
}

impl ApiEventMetric for BatchBlocklistUploadResponse {}
impl ApiEventMetric for BatchBlocklistJobStatusResponse {}
impl ApiEventMetric for ListBatchBlocklistJobsQuery {}
impl ApiEventMetric for ListBatchBlocklistJobsResponse {}
