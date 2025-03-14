//! Refunds V2 interface

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::RefundFlowData,
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::RefundsData,
    router_response_types::RefundsResponseData,
};

use crate::api::{ConnectorCommon, ConnectorIntegrationV2};

/// trait RefundExecuteV2
pub trait RefundExecuteV2:
    ConnectorIntegrationV2<Execute, RefundFlowData, RefundsData, RefundsResponseData>
{
}

/// trait RefundSyncV2
pub trait RefundSyncV2:
    ConnectorIntegrationV2<RSync, RefundFlowData, RefundsData, RefundsResponseData>
{
}

/// trait RefundV2
pub trait RefundV2: ConnectorCommon + RefundExecuteV2 + RefundSyncV2 {}
