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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DisputeSyncResponse {
    pub object_reference_id: api_models::webhooks::ObjectReferenceId,
    pub amount: common_utils::types::StringMinorUnit,
    pub currency: common_enums::enums::Currency,
    pub dispute_stage: common_enums::enums::DisputeStage,
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<time::PrimitiveDateTime>,
    pub created_at: Option<time::PrimitiveDateTime>,
    pub updated_at: Option<time::PrimitiveDateTime>,
}

pub type FetchDisputesResponse = Vec<DisputeSyncResponse>;
