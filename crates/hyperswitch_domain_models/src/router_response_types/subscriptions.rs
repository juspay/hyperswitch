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
