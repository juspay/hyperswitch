#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
