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

/// trait Refund
pub trait Refund: ConnectorCommon + RefundExecute + RefundSync {
    /// Returns the direct connector integration for voiding a successful refund.
    ///
    /// This is optional because most connectors do not support this operation. A connector that
    /// supports it can override this method and return its `VoidPostRefund` integration.
    fn get_void_post_refund_integration(
        &self,
    ) -> Option<api::BoxedConnectorIntegration<'_, VoidPostRefund, RefundsData, RefundsResponseData>>
    {
        None
    }
}
