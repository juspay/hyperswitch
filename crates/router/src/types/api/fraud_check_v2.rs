pub use hyperswitch_domain_models::router_flow_types::fraud_check::{
    Checkout, Fulfillment, RecordReturn, Sale, Transaction,
};
pub use hyperswitch_interfaces::api::fraud_check_v2::{
    FraudCheckCheckoutV2, FraudCheckFulfillmentV2, FraudCheckRecordReturnV2, FraudCheckSaleV2,
    FraudCheckTransactionV2,
};

use crate::types;

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
