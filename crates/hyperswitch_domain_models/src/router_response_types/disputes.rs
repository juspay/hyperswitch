#[derive(Default, Clone, Debug)]
pub struct AcceptDisputeResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

#[derive(Default, Clone, Debug)]
pub struct SubmitEvidenceResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct DefendDisputeResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

pub struct FileInfo {
    pub file_data: Option<Vec<u8>>,
    pub provider_file_id: Option<String>,
    pub file_type: Option<String>,
}
