#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest;

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPricesRequest {
    pub item_id: String,
}
