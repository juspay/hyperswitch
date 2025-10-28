use api_models::payments::Address;
use common_utils::id_type;

use crate::connector_endpoints;

#[derive(Debug, Clone)]
pub struct SubscriptionItem {
    pub item_price_id: String,
    pub quantity: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionCreateRequest {
    pub customer_id: id_type::CustomerId,
    pub subscription_id: id_type::SubscriptionId,
    pub subscription_items: Vec<SubscriptionItem>,
    pub billing_address: Address,
    pub auto_collection: SubscriptionAutoCollection,
    pub connector_params: connector_endpoints::ConnectorParams,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionAutoCollection {
    On,
    Off,
}
#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl GetSubscriptionPlansRequest {
    pub fn new(limit: Option<u32>, offset: Option<u32>) -> Self {
        Self { limit, offset }
    }
}

impl Default for GetSubscriptionPlansRequest {
    fn default() -> Self {
        Self {
            limit: Some(10),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPricesRequest {
    pub plan_price_id: String,
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionEstimateRequest {
    pub price_id: String,
}
