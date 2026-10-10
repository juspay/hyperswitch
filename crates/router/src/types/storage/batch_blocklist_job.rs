use common_utils::id_type;
pub use diesel_models::batch_blocklist_job::{
    BatchBlocklistJob, BatchBlocklistJobNew, BatchBlocklistJobUpdate,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchBlocklistTrackingData {
    pub job_id: String,
    pub merchant_id: id_type::MerchantId,
    pub processor_merchant_id: Option<id_type::MerchantId>,
    pub chunk_total_count: u32,
    pub completed_chunks: Vec<u32>,
    pub created_by: Option<String>,
    pub profile_id: Option<id_type::ProfileId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistExportTrackingData {
    pub job_id: String,
    pub merchant_id: id_type::MerchantId,
    pub processor_merchant_id: Option<id_type::MerchantId>,
    pub profile_id: id_type::ProfileId,
    /// Fixes the selected blocklist across retries; old jobs exported payment entries.
    #[serde(default)]
    pub transaction_type: common_enums::BlocklistTransactionType,
    /// Bounds the scan, so a run spanning minutes still describes one moment.
    pub snapshot_at: time::PrimitiveDateTime,
    /// Written once the multipart upload starts, so a retry can discard the abandoned parts.
    pub upload_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistProfileCloneTrackingData {
    pub job_id: String,
    pub merchant_id: id_type::MerchantId,
    pub processor_merchant_id: Option<id_type::MerchantId>,
    pub source_profile_id: id_type::ProfileId,
    /// Copied in this order.
    pub target_profile_ids: Vec<id_type::ProfileId>,
    /// Index into `target_profile_ids` of the target being copied; earlier ones are settled.
    pub current_target: usize,
    /// Bounds the scan, so a run spanning minutes still copies one moment of the source.
    pub snapshot_at: time::PrimitiveDateTime,
    /// Each flow drains processor-scoped rows, then legacy rows, before advancing to the next flow.
    pub draining_legacy_rows: bool,
    /// Older trackers only contain payment rows; retain their strict fingerprint cursor on resume.
    #[serde(default)]
    pub transaction_type: common_enums::BlocklistTransactionType,
    /// Keyset cursor within the current flow and merchant pass. `""` starts at the beginning.
    pub last_fingerprint_id: String,
    /// Source rows fully handled by this target tracker. This advances with the cursor.
    pub processed_rows: i32,
}

#[derive(Debug, Clone)]
pub struct BlocklistProfileCloneTargetUpdate {
    pub status: common_enums::BatchBlocklistJobStatus,
    pub processed_rows: i32,
    pub error_message: Option<String>,
}

/// Job-level progress for a multi-target profile clone, one entry per target. The generic row
/// counters on the job stay at zero.
#[derive(Debug, Clone)]
pub struct BlocklistProfileCloneJobMetadata {
    pub targets: Vec<BlocklistProfileCloneTargetMetadata>,
}

#[derive(Debug, Clone)]
pub struct BlocklistProfileCloneTargetMetadata {
    pub profile_id: id_type::ProfileId,
    pub status: common_enums::BatchBlocklistJobStatus,
    pub processed_rows: i32,
    pub error_message: Option<String>,
}

impl From<diesel_models::batch_blocklist_job::BlocklistProfileCloneJobMetadata>
    for BlocklistProfileCloneJobMetadata
{
    fn from(
        metadata: diesel_models::batch_blocklist_job::BlocklistProfileCloneJobMetadata,
    ) -> Self {
        Self {
            targets: metadata.targets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<BlocklistProfileCloneJobMetadata>
    for diesel_models::batch_blocklist_job::BlocklistProfileCloneJobMetadata
{
    fn from(metadata: BlocklistProfileCloneJobMetadata) -> Self {
        Self {
            targets: metadata.targets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<diesel_models::batch_blocklist_job::BlocklistProfileCloneTargetMetadata>
    for BlocklistProfileCloneTargetMetadata
{
    fn from(
        target: diesel_models::batch_blocklist_job::BlocklistProfileCloneTargetMetadata,
    ) -> Self {
        Self {
            profile_id: target.profile_id,
            status: target.status,
            processed_rows: target.processed_rows,
            error_message: target.error_message,
        }
    }
}

impl From<BlocklistProfileCloneTargetMetadata>
    for diesel_models::batch_blocklist_job::BlocklistProfileCloneTargetMetadata
{
    fn from(target: BlocklistProfileCloneTargetMetadata) -> Self {
        Self {
            profile_id: target.profile_id,
            status: target.status,
            processed_rows: target.processed_rows,
            error_message: target.error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use common_enums::BlocklistTransactionType;
    use serde_json::json;

    use super::BlocklistExportTrackingData;

    fn old_export_job() -> serde_json::Value {
        json!({
            "job_id": "export_job",
            "merchant_id": "merchant_test",
            "processor_merchant_id": null,
            "profile_id": "pro_source",
            "snapshot_at": common_utils::date_time::now(),
            "upload_id": "previous_upload"
        })
    }

    #[test]
    fn old_export_jobs_resume_in_the_payment_blocklist() {
        let tracking: BlocklistExportTrackingData =
            serde_json::from_value(old_export_job()).unwrap();
        assert_eq!(tracking.transaction_type, BlocklistTransactionType::Payment);
        assert_eq!(tracking.upload_id.as_deref(), Some("previous_upload"));
    }

    #[test]
    fn export_retry_payload_preserves_payout_selection() {
        let mut tracking: BlocklistExportTrackingData =
            serde_json::from_value(old_export_job()).unwrap();
        tracking.transaction_type = BlocklistTransactionType::Payout;
        let retry = BlocklistExportTrackingData {
            upload_id: Some("retry_upload".to_owned()),
            ..tracking
        };
        let restored: BlocklistExportTrackingData =
            serde_json::from_value(serde_json::to_value(retry).unwrap()).unwrap();
        assert_eq!(restored.transaction_type, BlocklistTransactionType::Payout);
        assert_eq!(restored.upload_id.as_deref(), Some("retry_upload"));
    }
}
