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
