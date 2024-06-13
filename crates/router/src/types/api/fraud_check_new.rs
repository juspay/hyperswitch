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

pub trait FraudCheckSaleNew:
    api::ConnectorIntegrationNew<Sale, types::FrmFlowData, FraudCheckSaleData, FraudCheckResponseData>
{
}

pub trait FraudCheckCheckoutNew:
    api::ConnectorIntegrationNew<
    Checkout,
    types::FrmFlowData,
    FraudCheckCheckoutData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckTransactionNew:
    api::ConnectorIntegrationNew<
    Transaction,
    types::FrmFlowData,
    FraudCheckTransactionData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckFulfillmentNew:
    api::ConnectorIntegrationNew<
    Fulfillment,
    types::FrmFlowData,
    FraudCheckFulfillmentData,
    FraudCheckResponseData,
>
{
}

pub trait FraudCheckRecordReturnNew:
    api::ConnectorIntegrationNew<
    RecordReturn,
    types::FrmFlowData,
    FraudCheckRecordReturnData,
    FraudCheckResponseData,
>
{
}

#[cfg(feature = "frm")]
pub trait FraudCheckNew:
    types::api::ConnectorCommon
    + FraudCheckSaleNew
    + FraudCheckTransactionNew
    + FraudCheckCheckoutNew
    + FraudCheckFulfillmentNew
    + FraudCheckRecordReturnNew
{
}
