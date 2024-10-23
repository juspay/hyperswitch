use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{AccessTokenAuth, Capture, PSync, Void},
    router_request_types::{
        AccessTokenRequestData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        PayoutsData, RefundsData,
    },
    router_response_types::{PaymentsResponseData, PayoutsResponseData, RefundsResponseData},
};

pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;

pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;
pub(crate) type RefreshTokenRouterData =
    RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
#[cfg(feature = "payouts")]
pub type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
