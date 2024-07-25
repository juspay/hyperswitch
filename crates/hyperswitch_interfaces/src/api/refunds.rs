//! Refunds interface

use hyperswitch_domain_models::{
    router_flow_types::{Execute, RSync},
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

/// trait Refund
pub trait Refund: ConnectorCommon + RefundExecute + RefundSync {}
