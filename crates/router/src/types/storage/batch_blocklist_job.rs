use common_utils::id_type;
pub use diesel_models::batch_blocklist_job::{
    BatchBlocklistJob, BatchBlocklistJobNew, BatchBlocklistJobUpdate,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchBlocklistTrackingData {
    pub job_id: String,
    pub merchant_id: id_type::MerchantId,
    pub chunk_total_count: u32,
    pub completed_chunks: Vec<u32>,
}
