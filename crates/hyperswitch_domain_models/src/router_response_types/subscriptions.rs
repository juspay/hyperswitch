use common_enums::Currency;

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansResponse {
    pub list: Vec<SubscriptionPlans>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlans {
    pub plan_id: String,
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
    pub amount: common_utils::types::MinorUnit,
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
