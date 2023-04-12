use actix_web::web::Bytes;
use utoipa::ToSchema;

#[derive(Debug, ToSchema)]
pub struct CreateFileRequest {
    /// File in Binary
    pub file: Option<Vec<Bytes>>,
    /// File name
    pub file_name: Option<String>,
    /// File size
    pub file_size: f64,
    /// File type,
    pub file_type: Option<mime::Mime>,
    /// Purpose of the file upload
    pub purpose: FilePurpose,
    /// Dispute id
    pub dispute_id: Option<String>,
    // pub dispute: Option<DisputeParams>,
}

#[derive(Debug, ToSchema, serde::Deserialize)]
pub struct DisputeParams {
    pub dispute_id: String,
}

#[derive(Debug, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FilePurpose {
    DisputeEvidence,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CreateFileResponse {
    /// ID of the file created
    pub file_id: String,
}
