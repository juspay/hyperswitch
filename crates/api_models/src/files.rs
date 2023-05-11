use utoipa::ToSchema;

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CreateFileResponse {
    /// ID of the file created
    pub file_id: String,
}
