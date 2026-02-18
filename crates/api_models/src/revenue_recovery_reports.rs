use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct RevenueRecoveryReportMetadata {
    pub file_name: String,
    pub timeline: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RevenueRecoveryReportUploadResponse {
    pub file_id: String,
    pub s3_key: String,
    pub status: String,
    pub uploaded_at: String,
    pub merchant_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UploadStatus {
    Uploading,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadStatusData {
    pub file_id: String,
    pub status: UploadStatus,
    pub s3_key: Option<String>,
    pub error: Option<String>,
    pub uploaded_at: String,
    pub completed_at: Option<String>,
    pub merchant_id: String,
}

#[derive(Debug, Serialize)]
pub struct RevenueRecoveryReportStatusResponse {
    pub file_id: String,
    pub status: UploadStatus,
    pub s3_key: Option<String>,
    pub error: Option<String>,
    pub uploaded_at: String,
    pub completed_at: Option<String>,
    pub merchant_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletedPart {
    #[serde(rename = "PartNumber")]
    pub part_number: i32,
    #[serde(rename = "ETag")]
    pub e_tag: String,
}
