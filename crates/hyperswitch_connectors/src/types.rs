use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::AccessTokenAuth,
    router_request_types::{AccessTokenRequestData, RefundsData},
    router_response_types::RefundsResponseData,
};

pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;
pub(crate) type RefreshTokenRouterData =
    RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
