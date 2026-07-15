use utoipa::ToSchema;

/// A multipart/form-data request containing a file upload
#[derive(Debug, ToSchema)]
pub struct MultipartRequestWithFile {
    /// The file to upload
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CreateFileResponse {
    /// ID of the file created
    pub file_id: String,
}

#[derive(Debug, serde::Serialize, ToSchema, Clone)]
pub struct FileMetadataResponse {
    /// ID of the file created
    pub file_id: String,
    /// Name of the file
    pub file_name: Option<String>,
    /// Size of the file
    pub file_size: i32,
    /// Type of the file
    pub file_type: String,
    /// File availability
    pub available: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct FileRetrieveQuery {
    ///Dispute Id
    pub dispute_id: Option<String>,
}
