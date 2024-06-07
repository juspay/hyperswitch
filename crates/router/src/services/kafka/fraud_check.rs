
// use diesel_models::enums as storage_enums;
use diesel_models::fraud_check::FraudCheck;
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;
use diesel_models::{
    enums::{FraudCheckLastStep, FraudCheckStatus, FraudCheckType},
};
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaFraudCheck<'a> {
    pub frm_id: &'a String,
    pub payment_id: &'a String,
    pub merchant_id: &'a String,
    pub attempt_id: &'a String,
    pub created_at: PrimitiveDateTime,
    pub frm_name: &'a String,
    pub frm_transaction_id: Option<&'a String,>,
    pub frm_transaction_type: FraudCheckType,
    pub frm_status: FraudCheckStatus,
    pub frm_score: Option<i32>,
    pub frm_reason: Option<serde_json::Value>,
    pub frm_error: Option<&'a String,>,
    pub payment_details: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
    pub last_step: FraudCheckLastStep,
    pub payment_capture_method: Option<storage_enums::CaptureMethod>, // In postFrm, we are updating capture method from automatic to manual. To store the merchant actual capture method, we are storing the actual capture method in payment_capture_method. It will be useful while approving the FRM decision.
}

impl<'a> KafkaFraudCheck<'a> {
    pub fn from_storage(check: &'a FraudCheck) -> Self {
        Self {
            frm_id: &check.frm_id,
            payment_id: &check.payment_id,
            merchant_id: &check.merchant_id,
            attempt_id: &check.attempt_id,
            created_at: check.created_at,
            frm_name: &check.frm_name,
            frm_transaction_id:  check.frm_transaction_id.as_ref(),
            frm_transaction_type: check.frm_transaction_type,
            frm_status: check.frm_status,
            frm_score:  check.frm_score,
            frm_reason: check.frm_reason.clone(),
            frm_error:  check.frm_error.as_ref(),
            payment_details:  check.payment_details.clone(),
            metadata:  check.metadata.clone(),
            modified_at: check.modified_at,
            last_step: check.last_step,
            payment_capture_method:  check.payment_capture_method,
        }
    }
}

impl<'a> super::KafkaMessage for KafkaFraudCheck<'a> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id, self.payment_id, self.attempt_id
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::FraudCheck
    }
}
