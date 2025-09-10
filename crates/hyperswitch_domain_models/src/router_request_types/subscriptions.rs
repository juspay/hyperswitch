#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest;

#[derive(Debug, Clone)]
pub struct GetSubscriptionEstimateRequest {
    pub price_id: String,
}
