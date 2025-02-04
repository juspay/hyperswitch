pub use hyperswitch_domain_models::{
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData, RefundMethod,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};

use crate::{
    services,
    types::{api, ErrorResponse, RouterData},
};

pub type FrmSaleRouterData = RouterData<api::Sale, FraudCheckSaleData, FraudCheckResponseData>;

pub type FrmSaleType =
    dyn services::ConnectorIntegration<api::Sale, FraudCheckSaleData, FraudCheckResponseData>;

#[derive(Debug, Clone)]
pub struct FrmRouterData {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector: String,
    // TODO: change this to PaymentId type
    pub payment_id: String,
    pub attempt_id: String,
    pub request: FrmRequest,
    pub response: FrmResponse,
}
#[derive(Debug, Clone)]
pub enum FrmRequest {
    Sale(FraudCheckSaleData),
    Checkout(Box<FraudCheckCheckoutData>),
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

pub type FrmCheckoutRouterData =
    RouterData<api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;

pub type FrmCheckoutType = dyn services::ConnectorIntegration<
    api::Checkout,
    FraudCheckCheckoutData,
    FraudCheckResponseData,
>;

pub type FrmTransactionRouterData =
    RouterData<api::Transaction, FraudCheckTransactionData, FraudCheckResponseData>;

pub type FrmTransactionType = dyn services::ConnectorIntegration<
    api::Transaction,
    FraudCheckTransactionData,
    FraudCheckResponseData,
>;

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
