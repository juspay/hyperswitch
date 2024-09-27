//! FRM V2 interface
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::FrmFlowData,
    router_flow_types::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};

use crate::{api::ConnectorIntegrationV2, errors};

/// trait FraudCheckSaleV2
pub trait FraudCheckSaleV2:
    ConnectorIntegrationV2<
    Sale,
    FrmFlowData,
    FraudCheckSaleData,
    FraudCheckResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait FraudCheckCheckoutV2
pub trait FraudCheckCheckoutV2:
    ConnectorIntegrationV2<
    Checkout,
    FrmFlowData,
    FraudCheckCheckoutData,
    FraudCheckResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait FraudCheckTransactionV2
pub trait FraudCheckTransactionV2:
    ConnectorIntegrationV2<
    Transaction,
    FrmFlowData,
    FraudCheckTransactionData,
    FraudCheckResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait FraudCheckFulfillmentV2
pub trait FraudCheckFulfillmentV2:
    ConnectorIntegrationV2<
    Fulfillment,
    FrmFlowData,
    FraudCheckFulfillmentData,
    FraudCheckResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait FraudCheckRecordReturnV2
pub trait FraudCheckRecordReturnV2:
    ConnectorIntegrationV2<
    RecordReturn,
    FrmFlowData,
    FraudCheckRecordReturnData,
    FraudCheckResponseData,
    Error = errors::ConnectorError,
>
{
}
