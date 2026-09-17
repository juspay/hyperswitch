use std::collections::BTreeMap;

use common_enums::enums;
use common_utils::events::ApiEventMetric;
use hyperswitch_masking::StrongSecret;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum BlocklistRequest {
    /// Deprecated, use `generic_card_bin`. A card number prefix of exactly 6 digits.
    #[deprecated(note = "use GenericCardBin, which accepts 6 to 10 digits")]
    CardBin(String),
    Fingerprint(String),
    /// Deprecated, use `generic_card_bin`. A card number prefix of exactly 8 digits.
    #[deprecated(note = "use GenericCardBin, which accepts 6 to 10 digits")]
    ExtendedCardBin(String),
    /// A card number prefix of 6 to 10 digits.
    GenericCardBin(String),
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
    #[schema(value_type = Option<String>, example = "pro_abcdefghijklmnop")]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistCountQuery {
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistCountResponse {
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    /// The total number of blocked entries for the given data_kind
    pub total_count: usize,
    /// The number of blocked entries for each BIN length, keyed by the BIN length.
    ///
    /// For `generic_card_bin`, this also includes entries blocked as `card_bin` or
    /// `extended_card_bin`. Omitted for `payment_method`, since fingerprints have a fixed length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts_by_length: Option<BTreeMap<usize, usize>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistLookupQuery {
    /// The raw value to check against the blocklist, e.g. a card BIN. Limited to 20 characters,
    /// the longest value a `fingerprint_id` can hold.
    #[schema(value_type = String, max_length = 20)]
    pub data: common_utils::types::BlocklistLookupData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistLookupResponse {
    pub data: String,
    /// Whether an entry matching `data` exists in the blocklist, under any data_kind
    pub blocked: bool,
}

impl ApiEventMetric for BlocklistCountQuery {}
impl ApiEventMetric for BlocklistCountResponse {}
impl ApiEventMetric for BlocklistLookupQuery {}
impl ApiEventMetric for BlocklistLookupResponse {}

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

/// One batch blocklist job, as returned by the listing and by `GET /blocklist/batch/{job_id}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistJobStatusResponse {
    pub job_id: String,
    pub merchant_id: String,
    /// Whether this job imported entries or exported them.
    #[schema(value_type = BatchBlocklistJobType)]
    pub job_type: enums::BatchBlocklistJobType,
    /// The file the merchant uploaded, or the name the export downloads as.
    pub file_name: Option<String>,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    pub total_rows: u32,
    pub succeeded_rows: u32,
    pub failed_rows: u32,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: time::PrimitiveDateTime,
    /// Exports only: when the stored file is removed by the storage lifecycle rule.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<time::PrimitiveDateTime>,
    /// Whether this job's file can be fetched right now.
    pub downloadable: bool,
    pub error_message: Option<String>,
    /// Exports only: short-lived signed link, generated per request and never stored. Null in the
    /// listing, which does not mint a link per row.
    #[schema(value_type = Option<String>)]
    pub download_url: Option<hyperswitch_masking::Secret<url::Url>>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub download_url_expires_at: Option<time::PrimitiveDateTime>,
}

/// Query parameters for listing batch blocklist jobs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBatchBlocklistJobsQuery {
    /// Limit on the number of objects to return
    #[serde(default)]
    pub limit: common_utils::types::list::PageSize,
    /// The starting point within a list of objects
    #[serde(default)]
    pub offset: common_utils::types::list::PageOffset,
    /// Restricts the listing to one kind of job. Both uploads and exports are returned when omitted.
    #[schema(value_type = Option<BatchBlocklistJobType>)]
    pub job_type: Option<enums::BatchBlocklistJobType>,
}

/// Response for `GET /blocklist/batch`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ListBatchBlocklistJobsResponse {
    pub count: usize,
    pub total_count: usize,
    pub data: Vec<BatchBlocklistJobStatusResponse>,
}

// ---- Blocklist CSV export types ----

/// Response for `POST /blocklist/export`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistExportResponse {
    /// Present for support and log correlation.
    pub export_id: String,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    /// The name this export will download as.
    pub file_name: String,
}

impl ApiEventMetric for BatchBlocklistUploadResponse {}
impl ApiEventMetric for BatchBlocklistJobStatusResponse {}
impl ApiEventMetric for ListBatchBlocklistJobsQuery {}
impl ApiEventMetric for ListBatchBlocklistJobsResponse {}
impl ApiEventMetric for BlocklistExportResponse {}
