use common_enums::Currency;
use common_utils::{id_type, types::MinorUnit};
use time::PrimitiveDateTime;

#[derive(Debug, Clone)]
pub struct SubscriptionCreateResponse {
    pub subscription_id: id_type::SubscriptionId,
    pub status: SubscriptionStatus,
    pub customer_id: id_type::CustomerId,
    pub currency_code: Currency,
    pub total_amount: MinorUnit,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
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

impl From<SubscriptionStatus> for api_models::subscription::SubscriptionStatus {
    fn from(status: SubscriptionStatus) -> Self {
        match status {
            SubscriptionStatus::Pending => Self::Pending,
            SubscriptionStatus::Trial => Self::Active,
            SubscriptionStatus::Active => Self::Active,
            SubscriptionStatus::Paused => Self::InActive,
            SubscriptionStatus::Unpaid => Self::InActive,
            SubscriptionStatus::Onetime => Self::Active,
            SubscriptionStatus::Cancelled => Self::InActive,
            SubscriptionStatus::Failed => Self::InActive,
        }
    }
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

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPricesResponse {
    pub list: Vec<SubscriptionPlanPrices>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlanPrices {
    pub price_id: String,
    pub plan_id: Option<String>,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub interval: PeriodUnit,
    pub interval_count: i64,
    pub trial_period: Option<i64>,
    pub trial_period_unit: Option<PeriodUnit>,
}

#[derive(Debug, Clone)]
pub enum PeriodUnit {
    Day,
    Week,
    Month,
    Year,
}
