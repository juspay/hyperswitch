use std::collections::{BTreeMap, HashSet};

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

/// A tagged single-entry request with an optional transaction namespace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistEntryRequest {
    #[serde(flatten)]
    pub entry: BlocklistRequest,
    /// Only `payment` and `payout` are supported. Defaults to `payment`.
    #[serde(default)]
    #[schema(value_type = BlocklistTransactionType)]
    pub transaction_type: common_enums::BlocklistTransactionType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistResponse {
    pub fingerprint_id: String,
    #[schema(value_type = BlocklistDataKind)]
    pub data_kind: enums::BlocklistDataKind,
    #[schema(value_type = BlocklistTransactionType)]
    pub transaction_type: common_enums::BlocklistTransactionType,
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
    /// Only `payment` and `payout` are supported. Defaults to `payment`.
    #[serde(default)]
    #[schema(value_type = BlocklistTransactionType)]
    pub transaction_type: common_enums::BlocklistTransactionType,
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
    /// Counts only `payment` or `payout` when selected. When omitted, counts both,
    /// including legacy NULL entries, which belong to `payment` when filtered.
    #[schema(value_type = Option<BlocklistTransactionType>)]
    pub transaction_type: Option<common_enums::BlocklistTransactionType>,
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
    /// Looks up only `payment` or `payout` when selected. Omission looks up both.
    #[schema(value_type = Option<BlocklistTransactionType>)]
    pub transaction_type: Option<common_enums::BlocklistTransactionType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistLookupResponse {
    pub data: String,
    /// Whether any entry matches within the resolved profile and selected transaction namespaces.
    pub blocked: bool,
    /// Distinct transaction types that block `data`, empty when `blocked` is false.
    #[schema(value_type = Vec<BlocklistTransactionType>)]
    pub transaction_types: Vec<common_enums::BlocklistTransactionType>,
}

impl ApiEventMetric for BlocklistCountQuery {}
impl ApiEventMetric for BlocklistCountResponse {}
impl ApiEventMetric for BlocklistLookupQuery {}
impl ApiEventMetric for BlocklistLookupResponse {}

impl ApiEventMetric for BlocklistRequest {}
impl ApiEventMetric for BlocklistEntryRequest {}
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BatchBlocklistUploadQuery {
    /// Overrides every CSV row when selected; omission preserves row selections.
    #[schema(value_type = Option<BlocklistTransactionType>)]
    pub transaction_type: Option<common_enums::BlocklistTransactionType>,
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
    /// The profile that owns this job; for a clone job, the profile whose entries are copied.
    #[schema(value_type = Option<String>, example = "pro_abcdefghijklmnop")]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// Whether this job imported, exported, or cloned entries.
    #[schema(value_type = BatchBlocklistJobType)]
    pub job_type: enums::BatchBlocklistJobType,
    /// The file the merchant uploaded, or the name the export downloads as. Clone jobs return
    /// `null`.
    pub file_name: Option<String>,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    pub total_rows: u32,
    pub succeeded_rows: u32,
    pub failed_rows: u32,
    /// Per-target status and row progress of a profile-clone job. Upload and export jobs return
    /// `null`.
    pub metadata: Option<ProfileCloneJobMetadata>,
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
    /// Restricts the listing to one kind of job. All kinds are returned when omitted.
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

/// Query parameters for `POST /blocklist/export`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BlocklistExportQuery {
    /// Exports only `payment` or `payout` entries when selected. Omission exports both.
    #[schema(value_type = Option<BlocklistTransactionType>)]
    pub transaction_type: Option<common_enums::BlocklistTransactionType>,
}

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

impl ApiEventMetric for BatchBlocklistUploadQuery {}
impl ApiEventMetric for BatchBlocklistUploadResponse {}
impl ApiEventMetric for BatchBlocklistJobStatusResponse {}
impl ApiEventMetric for ListBatchBlocklistJobsQuery {}
impl ApiEventMetric for ListBatchBlocklistJobsResponse {}
impl ApiEventMetric for BlocklistExportResponse {}
impl ApiEventMetric for BlocklistExportQuery {}

/// Progress of a profile-clone job across its target profiles.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ProfileCloneJobMetadata {
    pub targets: Vec<ProfileCloneTargetMetadata>,
}

/// Progress of one target profile within a profile-clone job.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ProfileCloneTargetMetadata {
    #[schema(value_type = String, example = "pro_abcdefghijklmnop")]
    pub profile_id: common_utils::id_type::ProfileId,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    /// Source entries handled so far for this target.
    pub processed_rows: u32,
    /// Present when this target's clone failed.
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CloneBlocklistEntriesRequest {
    #[schema(value_type = Vec<String>)]
    pub target_profile_ids: HashSet<common_utils::id_type::ProfileId>,
}

impl CloneBlocklistEntriesRequest {
    /// The checks that need nothing but the request and its source; whether each profile belongs
    /// to the merchant is settled later, against the merchant's profiles.
    pub fn validate(
        &self,
        source_profile_id: &common_utils::id_type::ProfileId,
    ) -> Result<(), String> {
        (!self.target_profile_ids.is_empty())
            .then_some(())
            .ok_or_else(|| "target_profile_ids must contain at least one profile".to_string())?;

        (!self.target_profile_ids.contains(source_profile_id))
            .then_some(())
            .ok_or_else(|| "target_profile_ids must not contain the source profile".to_string())
    }
}

/// Response for `POST /blocklist/clone`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CloneBlocklistEntriesResponse {
    #[schema(value_type = String, example = "pro_abcdefghijklmnop")]
    pub source_profile_id: common_utils::id_type::ProfileId,
    pub job_id: String,
    #[schema(value_type = BatchBlocklistJobStatus)]
    pub status: enums::BatchBlocklistJobStatus,
    #[schema(value_type = Vec<String>)]
    pub target_profile_ids: Vec<common_utils::id_type::ProfileId>,
}

impl ApiEventMetric for CloneBlocklistEntriesRequest {}
impl ApiEventMetric for CloneBlocklistEntriesResponse {}

#[cfg(test)]
mod tests {
    use common_enums::BlocklistTransactionType;
    use serde_json::json;

    use super::{
        BlocklistCountQuery, BlocklistEntryRequest, BlocklistLookupQuery, ListBlocklistQuery,
    };

    #[test]
    fn omitted_selection_preserves_management_scopes() {
        let single: BlocklistEntryRequest =
            serde_json::from_value(json!({"type": "generic_card_bin", "data": "424242"})).unwrap();
        let list: ListBlocklistQuery =
            serde_json::from_value(json!({"data_kind": "generic_card_bin"})).unwrap();
        let count: BlocklistCountQuery =
            serde_json::from_value(json!({"data_kind": "generic_card_bin"})).unwrap();
        let lookup: BlocklistLookupQuery =
            serde_json::from_value(json!({"data": "424242"})).unwrap();

        assert_eq!(single.transaction_type, BlocklistTransactionType::Payment);
        assert_eq!(list.transaction_type, BlocklistTransactionType::Payment);
        assert_eq!(count.transaction_type, None);
        assert_eq!(lookup.transaction_type, None);
    }

    #[test]
    fn payout_selection_is_retained_by_management_requests_and_queries() {
        let single: BlocklistEntryRequest = serde_json::from_value(
            json!({"type": "generic_card_bin", "data": "424242", "transaction_type": "payout"}),
        )
        .unwrap();
        let list: ListBlocklistQuery = serde_json::from_value(
            json!({"data_kind": "generic_card_bin", "transaction_type": "payout"}),
        )
        .unwrap();
        let count: BlocklistCountQuery = serde_json::from_value(
            json!({"data_kind": "generic_card_bin", "transaction_type": "payout"}),
        )
        .unwrap();
        let lookup: BlocklistLookupQuery =
            serde_json::from_value(json!({"data": "424242", "transaction_type": "payout"}))
                .unwrap();

        assert_eq!(single.transaction_type, BlocklistTransactionType::Payout);
        assert_eq!(list.transaction_type, BlocklistTransactionType::Payout);
        assert_eq!(
            count.transaction_type,
            Some(BlocklistTransactionType::Payout)
        );
        assert_eq!(
            lookup.transaction_type,
            Some(BlocklistTransactionType::Payout)
        );
    }

    #[test]
    fn payment_selection_is_retained_by_count_query() {
        let count: BlocklistCountQuery = serde_json::from_value(
            json!({"data_kind": "generic_card_bin", "transaction_type": "payment"}),
        )
        .unwrap();

        assert_eq!(
            count.transaction_type,
            Some(BlocklistTransactionType::Payment)
        );
    }

    #[test]
    fn unsupported_selection_is_rejected_instead_of_defaulting() {
        for selection in ["three_ds_authentication", "unknown", ""] {
            assert!(serde_json::from_value::<BlocklistEntryRequest>(
                json!({"type": "generic_card_bin", "data": "424242", "transaction_type": selection}),
            )
            .is_err());
            assert!(serde_json::from_value::<ListBlocklistQuery>(
                json!({"data_kind": "generic_card_bin", "transaction_type": selection}),
            )
            .is_err());
            assert!(serde_json::from_value::<BlocklistCountQuery>(
                json!({"data_kind": "generic_card_bin", "transaction_type": selection}),
            )
            .is_err());
            assert!(serde_json::from_value::<BlocklistLookupQuery>(
                json!({"data": "424242", "transaction_type": selection}),
            )
            .is_err());
        }
    }

    #[test]
    fn legacy_single_entry_bodies_preserve_all_tagged_kinds_and_data() {
        for (kind, data) in [
            ("card_bin", "424242"),
            ("extended_card_bin", "42424242"),
            ("generic_card_bin", "4242424242"),
            ("fingerprint", "fingerprint_123"),
        ] {
            let legacy_body = json!({"type": kind, "data": data});
            let request: BlocklistEntryRequest =
                serde_json::from_value(legacy_body.clone()).unwrap();

            assert_eq!(request.transaction_type, BlocklistTransactionType::Payment);
            assert_eq!(serde_json::to_value(&request.entry).unwrap(), legacy_body);

            let payout_body = json!({
                "type": kind,
                "data": data,
                "transaction_type": "payout",
            });
            let request: BlocklistEntryRequest =
                serde_json::from_value(payout_body.clone()).unwrap();

            assert_eq!(request.transaction_type, BlocklistTransactionType::Payout);
            assert_eq!(serde_json::to_value(&request.entry).unwrap(), legacy_body);
            assert_eq!(serde_json::to_value(&request).unwrap(), payout_body);
        }
    }

    #[test]
    fn malformed_single_entry_bodies_are_rejected() {
        for body in [
            json!({}),
            json!({"data": "424242"}),
            json!({"type": "unknown", "data": "424242"}),
            json!({"entry": {"type": "generic_card_bin", "data": "424242"}}),
        ] {
            assert!(serde_json::from_value::<BlocklistEntryRequest>(body).is_err());
        }

        for kind in [
            "card_bin",
            "extended_card_bin",
            "generic_card_bin",
            "fingerprint",
        ] {
            let missing_data = json!({"type": kind});
            assert!(serde_json::from_value::<BlocklistEntryRequest>(missing_data).is_err());

            for data in [json!(null), json!(424242), json!([]), json!({})] {
                let body = json!({"type": kind, "data": data});
                assert!(serde_json::from_value::<BlocklistEntryRequest>(body).is_err());
            }
        }
    }
}
