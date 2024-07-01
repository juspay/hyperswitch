use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::RefundFlowData,
    router_flow_types::refunds::{Execute, RSync},
};

use super::ConnectorCommon;
use crate::{services::api, types};
pub trait RefundExecuteV2:
    api::ConnectorIntegrationV2<Execute, RefundFlowData, types::RefundsData, types::RefundsResponseData>
{
}

pub trait RefundSyncV2:
    api::ConnectorIntegrationV2<RSync, RefundFlowData, types::RefundsData, types::RefundsResponseData>
{
}

pub trait RefundV2: ConnectorCommon + RefundExecuteV2 + RefundSyncV2 {}
