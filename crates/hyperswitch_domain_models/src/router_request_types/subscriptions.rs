use api_models::payments::Address;

use crate::connector_endpoints;

#[derive(Debug, Clone)]
pub struct SubscriptionItem {
    pub item_price_id: String,
    pub quantity: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionCreateRequest {
    pub customer_id: String,
    pub subscription_id: String,
    pub subscription_items: Vec<SubscriptionItem>,
    pub billing_address: Address,
    pub auto_collection: String,
    pub connector_params: connector_endpoints::ConnectorParams,
}
#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
