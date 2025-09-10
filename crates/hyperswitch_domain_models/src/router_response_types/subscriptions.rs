use time::PrimitiveDateTime;
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
    pub sub_total: i64,
    pub total: i64,
    pub credits_applied: Option<i64>,
    pub amount_paid: Option<i64>,
    pub amount_due: Option<i64>,
    pub currency: common_enums::Currency,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub line_items: Vec<SubscriptionLineItem>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionLineItem {
    pub item_id: String,
    pub item_type: String,
    pub description: String,
    pub amount: i64,
    pub currency: common_enums::Currency,
    pub unit_amount: Option<i64>,
    pub quantity: i64,
    pub pricing_model: Option<String>,
}
