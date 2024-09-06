use crate::router_data_v2::{
    flow_common_types::{PaymentFlowData, PayoutFlowData, RefundFlowData},
    RouterDataV2,
};

pub type PaymentsRouterDataV2<Flow, Request, Response> =
    RouterDataV2<Flow, PaymentFlowData, Request, Response>;
pub type PayoutsRouterDataV2<Flow, Request, Response> =
    RouterDataV2<Flow, PayoutFlowData, Request, Response>;
pub type RefundsRouterDataV2<Flow, Request, Response> =
    RouterDataV2<Flow, RefundFlowData, Request, Response>;
