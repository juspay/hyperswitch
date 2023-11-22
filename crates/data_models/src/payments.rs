use common_utils::pii;
use time::PrimitiveDateTime;

pub mod payment_attempt;
pub mod payment_intent;

use common_enums as storage_enums;

use self::payment_attempt::PaymentAttempt;
use crate::RemoteStorageObject;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentIntent {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::IntentStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<String>,
    pub payment_link_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
}
