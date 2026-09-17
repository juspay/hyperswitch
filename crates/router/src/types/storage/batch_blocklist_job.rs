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
    /// The source is drained in two passes, the same split the export uses: rows carrying a
    /// `processor_merchant_id` first, then the legacy rows without one.
    pub draining_legacy_rows: bool,
    /// Keyset cursor within the current pass. `""` starts the pass from the beginning.
    pub last_fingerprint_id: String,
    /// Source rows fully handled by this target tracker. This advances with the cursor.
    pub processed_rows: i32,
}

/// Job-level progress for a multi-target profile clone, one entry per target. The generic row
/// counters on the job stay at zero.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistProfileCloneJobMetadata {
    pub targets: Vec<BlocklistProfileCloneTargetMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlocklistProfileCloneTargetMetadata {
    pub profile_id: id_type::ProfileId,
    pub status: common_enums::BatchBlocklistJobStatus,
    pub processed_rows: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlocklistProfileCloneTargetUpdate {
    pub status: common_enums::BatchBlocklistJobStatus,
    pub processed_rows: i32,
    pub error_message: Option<String>,
}
