#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPricesRequest {
    pub plan_price_id: String,
}
