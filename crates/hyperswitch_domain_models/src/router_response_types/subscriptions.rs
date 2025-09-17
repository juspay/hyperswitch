use time::PrimitiveDateTime;
use common_utils::types::MinorUnit;

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
pub struct GetSubscriptionEstimateResponse {
    pub sub_total: MinorUnit,
    pub total: MinorUnit,
    pub credits_applied: Option<MinorUnit>,
    pub amount_paid: Option<MinorUnit>,
    pub amount_due: Option<MinorUnit>,
    pub currency: common_enums::Currency,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub line_items: Vec<SubscriptionLineItem>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionLineItem {
    pub item_id: String,
    pub item_type: String,
    pub description: String,
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub unit_amount: Option<MinorUnit>,
    pub quantity: i64,
    pub pricing_model: Option<String>,
}
