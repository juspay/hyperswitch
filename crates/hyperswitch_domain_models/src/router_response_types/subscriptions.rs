use common_enums::enums;
use common_utils::types::MinorUnit;
use time::PrimitiveDateTime;
use common_utils::id_type;

#[derive(Debug, Clone)]
pub struct SubscriptionCreateResponse {
    pub subscription_id: String,
    pub status: SubscriptionStatus,
    pub customer_id: id_type::CustomerId,
    pub currency_code: enums::Currency,
    pub total_amount: MinorUnit,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Created,
    PaymentInProgress,
    Active,
    PaymentFailed,
    Cancelled,
    Expired,
    Paused,
    PendingConfirmation,
}
