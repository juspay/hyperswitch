use common_utils::pii::Email;

use crate::{
    connector::signifyd::transformers::RefundMethod,
    core::fraud_check::types::FrmFulfillmentRequest,
    pii::Serialize,
    services,
    types::{self, api, storage_enums, ErrorResponse, ResponseId, RouterData},
};

pub type FrmSaleRouterData = RouterData<api::Sale, FraudCheckSaleData, FraudCheckResponseData>;

pub type FrmSaleType =
    dyn services::ConnectorIntegration<api::Sale, FraudCheckSaleData, FraudCheckResponseData>;

#[derive(Debug, Clone)]
pub struct FraudCheckSaleData {
    pub amount: i64,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
}
#[derive(Debug, Clone)]
pub struct FrmRouterData {
    pub merchant_id: String,
    pub connector: String,
    pub payment_id: String,
    pub attempt_id: String,
    pub request: FrmRequest,
    pub response: FrmResponse,
}
#[derive(Debug, Clone)]
pub enum FrmRequest {
    Sale(FraudCheckSaleData),
    Checkout(FraudCheckCheckoutData),
    Transaction(FraudCheckTransactionData),
    Fulfillment(FraudCheckFulfillmentData),
    RecordReturn(FraudCheckRecordReturnData),
}
#[derive(Debug, Clone)]
pub enum FrmResponse {
    Sale(Result<FraudCheckResponseData, ErrorResponse>),
    Checkout(Result<FraudCheckResponseData, ErrorResponse>),
    Transaction(Result<FraudCheckResponseData, ErrorResponse>),
    Fulfillment(Result<FraudCheckResponseData, ErrorResponse>),
    RecordReturn(Result<FraudCheckResponseData, ErrorResponse>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FraudCheckResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        status: storage_enums::FraudCheckStatus,
        connector_metadata: Option<serde_json::Value>,
        reason: Option<serde_json::Value>,
        score: Option<i32>,
    },
    FulfillmentResponse {
        order_id: String,
        shipment_ids: Vec<String>,
    },
    RecordReturnResponse {
        resource_id: ResponseId,
        connector_metadata: Option<serde_json::Value>,
        return_id: Option<String>,
    },
}

pub type FrmCheckoutRouterData =
    RouterData<api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;

pub type FrmCheckoutType = dyn services::ConnectorIntegration<
    api::Checkout,
    FraudCheckCheckoutData,
    FraudCheckResponseData,
>;

#[derive(Debug, Clone)]
pub struct FraudCheckCheckoutData {
    pub amount: i64,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub currency: Option<common_enums::Currency>,
    pub browser_info: Option<types::BrowserInformation>,
    pub payment_method_data: Option<api_models::payments::AdditionalPaymentData>,
    pub email: Option<Email>,
    pub gateway: Option<String>,
}

pub type FrmTransactionRouterData =
    RouterData<api::Transaction, FraudCheckTransactionData, FraudCheckResponseData>;

pub type FrmTransactionType = dyn services::ConnectorIntegration<
    api::Transaction,
    FraudCheckTransactionData,
    FraudCheckResponseData,
>;

#[derive(Debug, Clone)]
pub struct FraudCheckTransactionData {
    pub amount: i64,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_transaction_id: Option<String>,
}

pub type FrmFulfillmentRouterData =
    RouterData<api::Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>;

pub type FrmFulfillmentType = dyn services::ConnectorIntegration<
    api::Fulfillment,
    FraudCheckFulfillmentData,
    FraudCheckResponseData,
>;
pub type FrmRecordReturnRouterData =
    RouterData<api::RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>;

pub type FrmRecordReturnType = dyn services::ConnectorIntegration<
    api::RecordReturn,
    FraudCheckRecordReturnData,
    FraudCheckResponseData,
>;

#[derive(Debug, Clone)]
pub struct FraudCheckFulfillmentData {
    pub amount: i64,
    pub order_details: Option<Vec<masking::Secret<serde_json::Value>>>,
    pub fulfillment_req: FrmFulfillmentRequest,
}

#[derive(Debug, Clone)]
pub struct FraudCheckRecordReturnData {
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub refund_method: RefundMethod,
    pub refund_transaction_id: Option<String>,
}
