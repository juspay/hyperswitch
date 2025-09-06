use api_models::payments::Address;
use common_enums::enums;

use crate::connector_endpoints;

#[derive(Debug, Clone)]
pub struct SubscriptionsRecordBackRequest {
    pub merchant_reference_id: String,
    pub amount: common_utils::types::MinorUnit,
    pub currency: enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,
    pub connector_params: connector_endpoints::ConnectorParams,
}

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
pub struct CreateCustomerRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub locale: Option<String>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Clone)]
pub struct BillingAddress {
    pub first_name: String,
    pub last_name: String,
    pub line1: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}
