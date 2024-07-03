use time::PrimitiveDateTime;

#[derive(Default, Debug)]
pub struct DisputePayload {
    pub amount: String,
    pub currency: String,
    pub dispute_stage: common_enums::enums::DisputeStage,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
}
