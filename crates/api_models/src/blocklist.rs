use common_enums::enums;
use common_utils::events::ApiEventMetric;
use hyperswitch_masking::StrongSecret;
use utoipa::ToSchema;

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
pub struct BatchBlocklistRowError {
    /// 0-based row index in the CSV (excluding header).
    pub row_index: usize,
    #[schema(value_type = BlocklistDataKind)]
    pub r#type: enums::BlocklistDataKind,
    pub data: String,
    pub reason: String,
}

/// Response body when a batch-upload request fails per-row validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistValidationError {
    pub errors: Vec<BatchBlocklistRowError>,
}

/// Response returned on a successful `POST /blocklist/batch`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistUploadResponse {
    pub job_id: String,
    pub total_rows: u32,
    pub status: enums::BatchBlocklistJobStatus,
}

/// Response for `GET /blocklist/batch/{job_id}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistJobStatusResponse {
    pub job_id: String,
    pub merchant_id: String,
    pub status: enums::BatchBlocklistJobStatus,
    pub total_rows: i32,
    pub succeeded_rows: i32,
    pub failed_rows: i32,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: time::PrimitiveDateTime,
}

/// Query parameters for listing batch blocklist jobs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBatchBlocklistJobsQuery {
    #[serde(default = "default_batch_list_limit")]
    pub limit: u16,
    #[serde(default)]
    pub offset: u32,
}

fn default_batch_list_limit() -> u16 {
    10
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
