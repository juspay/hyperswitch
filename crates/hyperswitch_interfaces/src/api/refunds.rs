//! Refunds interface

use hyperswitch_domain_models::{
    router_flow_types::{Execute, RSync, VoidPostRefund},
    router_request_types::RefundsData,
    router_response_types::RefundsResponseData,
};

use crate::api::{self, ConnectorCommon};

/// trait RefundExecute
pub trait RefundExecute:
    api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData>
{
}

/// trait RefundSync
pub trait RefundSync: api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> {}

/// trait RefundVoidPostRefund
pub trait RefundVoidPostRefund:
    api::ConnectorIntegration<VoidPostRefund, RefundsData, RefundsResponseData>
{
}

/// trait Refund
pub trait Refund: ConnectorCommon + RefundExecute + RefundSync + RefundVoidPostRefund {}
