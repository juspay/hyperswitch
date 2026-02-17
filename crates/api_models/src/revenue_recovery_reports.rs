use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct CompleteMultipartUploadRequest {
    pub file_id: String,
    pub upload_id: String,
    pub parts: Vec<CompletedPart>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletedPart {
    #[serde(rename = "PartNumber")]
    pub part_number: i32,
    #[serde(rename = "ETag")]
    pub e_tag: String,
}
