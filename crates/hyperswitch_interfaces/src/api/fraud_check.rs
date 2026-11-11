//! FRM interface
use hyperswitch_domain_models::{
    router_flow_types::{Checkout, Fulfillment, PoFrm, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckPayoutData,
        FraudCheckRecordReturnData, FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};

use crate::api::ConnectorIntegration;

/// trait FraudCheckSale
pub trait FraudCheckSale:
    ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData>
{
}

/// trait FraudCheckCheckout
pub trait FraudCheckCheckout:
    ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
{
}

/// trait FraudCheckTransaction
pub trait FraudCheckTransaction:
    ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
{
}

/// trait FraudCheckFulfillment
pub trait FraudCheckFulfillment:
    ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
}

/// trait FraudCheckRecordReturn
pub trait FraudCheckRecordReturn:
    ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
{
}

/// Trait for payout fraud checks.
pub trait FraudCheckPayout:
    ConnectorIntegration<PoFrm, FraudCheckPayoutData, FraudCheckResponseData>
{
}

/// trait FraudCheck
pub trait FraudCheck:
    super::ConnectorCommon
    + FraudCheckSale
    + FraudCheckTransaction
    + FraudCheckCheckout
    + FraudCheckFulfillment
    + FraudCheckRecordReturn
    + FraudCheckPayout
{
}
