pub use hyperswitch_domain_models::router_flow_types::fraud_check::{
    Checkout, Fulfillment, RecordReturn, Sale, Transaction,
};

use crate::{
    services::api,
    types::{
        self,
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
            FraudCheckResponseData, FraudCheckSaleData, FraudCheckTransactionData,
        },
    },
};

pub trait FraudCheckSaleV2:
    api::ConnectorIntegrationV2<Sale, types::FrmFlowData, FraudCheckSaleData, FraudCheckResponseData>
{
}

pub trait FraudCheckCheckoutV2:
    api::ConnectorIntegrationV2<
    Checkout,
    types::FrmFlowData,
    FraudCheckCheckoutData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckTransactionV2:
    api::ConnectorIntegrationV2<
    Transaction,
    types::FrmFlowData,
    FraudCheckTransactionData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckFulfillmentV2:
    api::ConnectorIntegrationV2<
    Fulfillment,
    types::FrmFlowData,
    FraudCheckFulfillmentData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckRecordReturnV2:
    api::ConnectorIntegrationV2<
    RecordReturn,
    types::FrmFlowData,
    FraudCheckRecordReturnData,
    FraudCheckResponseData,
>
{
}

#[cfg(feature = "frm")]
pub trait FraudCheckV2:
    types::api::ConnectorCommon
    + FraudCheckSaleV2
    + FraudCheckTransactionV2
    + FraudCheckCheckoutV2
    + FraudCheckFulfillmentV2
    + FraudCheckRecordReturnV2
{
}
