use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::{Capture, PSync, Void},
    router_request_types::{
        PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData, RefundsData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};

pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;

pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
