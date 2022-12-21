use storage_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::types::domain;

///
/// Domain type for PaymentAttempt
///
// #[derive(frunk::LabelledGeneric)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct PaymentAttempt {
    pub id: i32,
    pub payment_id: domain::PaymentId,
    pub merchant_id: String,
    pub attempt_id: domain::AttemptId,
    pub status: storage_enums::AttemptStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<i64>,
    pub surcharge_amount: Option<i64>,
    pub tax_amount: Option<i64>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<storage_enums::PaymentMethodType>,
    pub payment_flow: Option<storage_enums::PaymentFlow>,
    pub redirect: Option<bool>,
    pub connector_transaction_id: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<i64>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
}
