pub use api_models::refunds::{
    RefundRequest, RefundResponse, RefundStatus, RefundType, RefundUpdateRequest,
    RefundsRetrieveRequest,
};
pub use hyperswitch_domain_models::{
    router_data_new::flow_common_types::RefundFlowData,
    router_flow_types::refunds::{Execute, RSync},
};

use super::ConnectorCommon;
use crate::{services::api, types};
pub trait RefundExecuteNew:
    api::ConnectorIntegrationNew<
    Execute,
    RefundFlowData,
    types::RefundsData,
    types::RefundsResponseData,
>
{
}

pub trait RefundSyncNew:
    api::ConnectorIntegrationNew<RSync, RefundFlowData, types::RefundsData, types::RefundsResponseData>
{
}

pub trait RefundNew: ConnectorCommon + RefundExecuteNew + RefundSyncNew {}
