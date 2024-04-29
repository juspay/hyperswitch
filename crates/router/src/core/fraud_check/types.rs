use api_models::{
    enums as api_enums,
    enums::{PaymentMethod, PaymentMethodType},
    payments::Amount,
    refunds::RefundResponse,
};
use common_enums::FrmSuggestion;
use common_utils::pii::Email;
use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use masking::Serialize;
use serde::Deserialize;
use utoipa::ToSchema;

use super::operation::BoxedFraudCheckOperation;
use crate::{
    pii::Secret,
    types::{
        domain::MerchantAccount,
        storage::{enums as storage_enums, fraud_check::FraudCheck},
        PaymentAddress,
    },
};

#[derive(Clone, Default, Debug)]
pub struct PaymentIntentCore {
    pub payment_id: String,
}

#[derive(Clone, Debug)]
pub struct PaymentAttemptCore {
    pub attempt_id: String,
    pub payment_details: Option<PaymentDetails>,
    pub amount: Amount,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaymentDetails {
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method: Option<PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodType>,
    pub refund_transaction_id: Option<String>,
}
#[derive(Clone, Default, Debug)]
pub struct FrmMerchantAccount {
    pub merchant_id: String,
}

#[derive(Clone, Debug)]
pub struct FrmData {
    pub payment_intent: PaymentIntent,
    pub payment_attempt: PaymentAttempt,
    pub merchant_account: MerchantAccount,
    pub fraud_check: FraudCheck,
    pub address: PaymentAddress,
    pub connector_details: ConnectorDetailsCore,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub refund: Option<RefundResponse>,
    pub frm_metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct FrmInfo<F> {
    pub fraud_check_operation: BoxedFraudCheckOperation<F>,
    pub frm_data: Option<FrmData>,
    pub suggested_action: Option<FrmSuggestion>,
}

#[derive(Clone, Debug)]
pub struct ConnectorDetailsCore {
    pub connector_name: String,
    pub profile_id: String,
}
#[derive(Clone)]
pub struct PaymentToFrmData {
    pub amount: Amount,
    pub payment_intent: PaymentIntent,
    pub payment_attempt: PaymentAttempt,
    pub merchant_account: MerchantAccount,
    pub address: PaymentAddress,
    pub connector_details: ConnectorDetailsCore,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub frm_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrmConfigsObject {
    pub frm_enabled_pm: Option<PaymentMethod>,
    pub frm_enabled_pm_type: Option<PaymentMethodType>,
    pub frm_enabled_gateway: Option<api_models::enums::Connector>,
    pub frm_action: api_enums::FrmAction,
    pub frm_preferred_flow_type: api_enums::FrmPreferredFlowTypes,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiRequest {
    ///unique order_id for the order_details in the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///denotes the status of the fulfillment... can be one of PARTIAL, COMPLETE, REPLACEMENT, CANCELED
    #[schema(value_type = Option<FulfillmentStatus>, example = "COMPLETE")]
    pub fulfillment_status: Option<FulfillmentStatus>,
    ///contains details of the fulfillment
    #[schema(value_type = Vec<Fulfillments>)]
    pub fulfillments: Vec<Fulfillments>,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct FrmFulfillmentRequest {
    ///unique payment_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub payment_id: String,
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
    #[schema(max_length = 255, example = "track_8327446667")]
    pub tracking_number: Option<String>,
    //tracking_url for tracking the product
    pub tracking_url: Option<String>,
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

#[derive(Debug, ToSchema, Clone, Serialize)]
pub struct FrmFulfillmentResponse {
    ///unique order_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///shipment_ids used in the fulfillment overall...also data from previous fulfillments for the same transactions/order is sent
    #[schema(example = r#"["ship_101", "ship_102"]"#)]
    pub shipment_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiResponse {
    ///unique order_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///shipment_ids used in the fulfillment overall...also data from previous fulfillments for the same transactions/order is sent
    #[schema(example = r#"["ship_101","ship_102"]"#)]
    pub shipment_ids: Vec<String>,
}

pub const CANCEL_INITIATED: &str = "Cancel Initiated with the processor";
