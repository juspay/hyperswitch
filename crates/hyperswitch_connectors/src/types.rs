use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{AccessTokenAuth, Capture, PSync, PreProcessing, Void},
    router_request_types::{
        AccessTokenRequestData, PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData,
        PaymentsSyncData, RefundsData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};

pub(crate) type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;

pub(crate) type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;
pub(crate) type RefreshTokenRouterData =
    RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub(crate) type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;
pub(crate) type PaymentsPreprocessingResponseRouterData<R> =
    ResponseRouterData<PreProcessing, R, PaymentsPreProcessingData, PaymentsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
