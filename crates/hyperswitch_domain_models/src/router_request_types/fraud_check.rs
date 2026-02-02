use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    pii::Email,
};
use diesel_models::types::OrderDetailsWithAmount;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::router_request_types;
#[derive(Debug, Clone)]
pub struct FraudCheckSaleData {
    pub amount: i64,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub currency: Option<common_enums::Currency>,
    pub email: Option<Email>,
}

#[derive(Debug, Clone)]
pub struct FraudCheckCheckoutData {
    pub amount: i64,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub currency: Option<common_enums::Currency>,
    pub browser_info: Option<router_request_types::BrowserInformation>,
    pub payment_method_data: Option<api_models::payments::AdditionalPaymentData>,
    pub email: Option<Email>,
    pub gateway: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FraudCheckTransactionData {
    pub amount: i64,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub currency: Option<common_enums::Currency>,
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_transaction_id: Option<String>,
    //The name of the payment gateway or financial institution that processed the transaction.
    pub connector: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FraudCheckRecordReturnData {
    pub amount: i64,
    pub currency: Option<common_enums::Currency>,
    pub refund_method: RefundMethod,
    pub refund_transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundMethod {
    StoreCredit,
    OriginalPaymentInstrument,
    NewPaymentInstrument,
}

#[derive(Debug, Clone)]
pub struct FraudCheckFulfillmentData {
    pub amount: i64,
    pub order_details: Option<Vec<Secret<serde_json::Value>>>,
    pub fulfillment_req: FrmFulfillmentRequest,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct FrmFulfillmentRequest {
    ///unique payment_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub payment_id: common_utils::id_type::PaymentId,
    ///unique order_id for the order_details in the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///denotes the status of the fulfillment... can be one of PARTIAL, COMPLETE, REPLACEMENT, CANCELED
    #[schema(value_type = Option<FulfillmentStatus>, example = "COMPLETE")]
    pub fulfillment_status: Option<FulfillmentStatus>,
    ///contains details of the fulfillment
    #[schema(value_type = Vec<Fulfillments>)]
    pub fulfillments: Vec<Fulfillments>,
    //name of the tracking Company
    #[schema(max_length = 255, example = "fedex")]
    pub tracking_company: Option<String>,
    //tracking ID of the product
    #[schema(example = r#"["track_8327446667", "track_8327446668"]"#)]
    pub tracking_numbers: Option<Vec<String>>,
    //tracking_url for tracking the product
    pub tracking_urls: Option<Vec<String>>,
    // The name of the Shipper.
    pub carrier: Option<String>,
    // Fulfillment method for the shipment.
    pub fulfillment_method: Option<String>,
    // Statuses to indicate shipment state.
    pub shipment_status: Option<String>,
    // The date and time items are ready to be shipped.
    pub shipped_at: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct Fulfillments {
    ///shipment_id of the shipped items
    #[schema(max_length = 255, example = "ship_101")]
    pub shipment_id: String,
    ///products sent in the shipment
    #[schema(value_type = Option<Vec<Product>>)]
    pub products: Option<Vec<Product>>,
    ///destination address of the shipment
    #[schema(value_type = Destination)]
    pub destination: Destination,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub enum FulfillmentStatus {
    PARTIAL,
    COMPLETE,
    REPLACEMENT,
    CANCELED,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct Product {
    pub item_name: String,
    pub item_quantity: i64,
    pub item_id: String,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct Destination {
    pub full_name: Secret<String>,
    pub organization: Option<String>,
    pub email: Option<Email>,
    pub address: Address,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct Address {
    pub street_address: Secret<String>,
    pub unit: Option<Secret<String>>,
    pub postal_code: Secret<String>,
    pub city: String,
    pub province_code: Secret<String>,
    pub country_code: common_enums::CountryAlpha2,
}

impl ApiEventMetric for FrmFulfillmentRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::FraudCheck)
    }
}
