use common_enums::enums;
use common_utils::types::MinorUnit;
use time::PrimitiveDateTime;

#[derive(Debug, Clone)]
pub struct SubscriptionCreateResponse {
    pub subscription_id: String,
    pub invoice_id: String,
    pub status: String,
    pub customer_id: String,
    pub currency_code: enums::Currency,
    pub total_amount: MinorUnit,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
}
