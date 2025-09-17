use common_enums::enums;
use common_utils::{id_type, types::MinorUnit};
use time::PrimitiveDateTime;

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
    Pending,
    Trial,
    Active,
    Paused,
    Unpaid,
    Onetime,
    Cancelled,
    Failed,
}
#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansResponse {
    pub list: Vec<SubscriptionPlans>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlans {
    pub subscription_provider_plan_id: String,
    pub name: String,
    pub description: Option<String>,
}
