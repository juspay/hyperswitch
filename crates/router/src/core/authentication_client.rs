use api_models::authentication::{
    AuthenticationAuthenticateRequest, AuthenticationAuthenticateResponse,
    AuthenticationCreateRequest, AuthenticationEligibilityRequest,
    AuthenticationEligibilityResponse, AuthenticationResponse, AuthenticationSyncPostUpdateRequest,
    AuthenticationSyncRequest, AuthenticationSyncResponse,
};
use common_utils::request::{Headers, Method, RequestContent};
use hyperswitch_interfaces::{impl_microservice_flow, micro_service::MicroserviceClient};
use payment_methods::configs::AuthenticationServiceConfig;
use router_env::RequestIdentifier;
use serde::{Deserialize, Serialize};
use url::Url;

pub struct AuthenticationServiceClient<'a> {
    base_url: Url,
    parent_headers: Headers,
    trace: &'a RequestIdentifier,
}

impl<'a> AuthenticationServiceClient<'a> {
    pub fn new(
        config: &'a AuthenticationServiceConfig,
        trace: &'a RequestIdentifier,
    ) -> Result<Self, hyperswitch_interfaces::micro_service::MicroserviceClientError> {
        let base_url = config.base_url.0.clone();

        let mut parent_headers = Headers::new();
        parent_headers.insert((
            "api-key".to_string(),
            hyperswitch_masking::Maskable::new_masked(config.api_key.clone()),
        ));

        Ok(Self {
            base_url,
            parent_headers,
            trace,
        })
    }
}

impl MicroserviceClient for AuthenticationServiceClient<'_> {
    fn base_url(&self) -> &Url {
        &self.base_url
    }
    fn parent_headers(&self) -> &Headers {
        &self.parent_headers
    }
    fn trace(&self) -> &RequestIdentifier {
        self.trace
    }
}

// ----------------------------------------------------------------------------
// Flow: authentication_create
// ----------------------------------------------------------------------------
pub struct AuthenticationCreateFlow;

#[derive(Serialize)]
pub struct AuthCreateReq(pub AuthenticationCreateRequest);

#[derive(Deserialize)]
pub struct AuthCreateResp(pub AuthenticationResponse);

impl TryFrom<&AuthenticationCreateRequest> for AuthCreateReq {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(req: &AuthenticationCreateRequest) -> Result<Self, Self::Error> {
        Ok(Self(req.clone()))
    }
}

impl TryFrom<AuthCreateResp> for AuthenticationResponse {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(resp: AuthCreateResp) -> Result<Self, Self::Error> {
        Ok(resp.0)
    }
}

impl_microservice_flow!(
    AuthenticationCreateFlow,
    method = Method::Post,
    path = "/authentication",
    v1_request = AuthenticationCreateRequest,
    v2_request = AuthCreateReq,
    v2_response = AuthCreateResp,
    v1_response = AuthenticationResponse,
    client = AuthenticationServiceClient<'_>,
    body = |_, req: AuthCreateReq| Some(RequestContent::Json(Box::new(serde_json::json!(req.0))))
);

// ----------------------------------------------------------------------------
// Flow: authentication_eligibility
// ----------------------------------------------------------------------------
pub struct AuthenticationEligibilityFlow;

#[derive(Serialize)]
pub struct AuthEligibilityReq(pub AuthenticationEligibilityRequest);

#[derive(Deserialize)]
pub struct AuthEligibilityResp(pub AuthenticationEligibilityResponse);

impl TryFrom<&(AuthenticationEligibilityRequest, String)> for AuthEligibilityReq {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(req: &(AuthenticationEligibilityRequest, String)) -> Result<Self, Self::Error> {
        Ok(Self(req.0.clone()))
    }
}

impl TryFrom<AuthEligibilityResp> for AuthenticationEligibilityResponse {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(resp: AuthEligibilityResp) -> Result<Self, Self::Error> {
        Ok(resp.0)
    }
}

impl_microservice_flow!(
    AuthenticationEligibilityFlow,
    method = Method::Post,
    path = "/authentication/{authentication_id}/eligibility",
    v1_request = (AuthenticationEligibilityRequest, String), // passing auth_id separately for path
    v2_request = AuthEligibilityReq,
    v2_response = AuthEligibilityResp,
    v1_response = AuthenticationEligibilityResponse,
    client = AuthenticationServiceClient<'_>,
    body = |_, req: AuthEligibilityReq| Some(RequestContent::Json(Box::new(serde_json::json!(req.0)))),
    path_params = |_, req: &(AuthenticationEligibilityRequest, String)| vec![("authentication_id", req.1.clone())]
);

// ----------------------------------------------------------------------------
// Flow: authentication_authenticate
// ----------------------------------------------------------------------------
pub struct AuthenticationAuthenticateFlow;

#[derive(Serialize)]
pub struct AuthAuthenticateReq(pub AuthenticationAuthenticateRequest);

#[derive(Deserialize)]
pub struct AuthAuthenticateResp(pub AuthenticationAuthenticateResponse);

impl TryFrom<&AuthenticationAuthenticateRequest> for AuthAuthenticateReq {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(req: &AuthenticationAuthenticateRequest) -> Result<Self, Self::Error> {
        Ok(Self(req.clone()))
    }
}

impl TryFrom<AuthAuthenticateResp> for AuthenticationAuthenticateResponse {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(resp: AuthAuthenticateResp) -> Result<Self, Self::Error> {
        Ok(resp.0)
    }
}

impl_microservice_flow!(
    AuthenticationAuthenticateFlow,
    method = Method::Post,
    path = "/authentication/{authentication_id}/authenticate",
    v1_request = AuthenticationAuthenticateRequest,
    v2_request = AuthAuthenticateReq,
    v2_response = AuthAuthenticateResp,
    v1_response = AuthenticationAuthenticateResponse,
    client = AuthenticationServiceClient<'_>,
    body = |_, req: AuthAuthenticateReq| Some(RequestContent::Json(Box::new(serde_json::json!(req.0)))),
    path_params = |_, req: &AuthenticationAuthenticateRequest| vec![("authentication_id", req.authentication_id.clone().get_string_repr().to_owned())]
);

// ----------------------------------------------------------------------------
// Flow: authentication_sync_post_update
// ----------------------------------------------------------------------------
pub struct AuthenticationSyncPostUpdateFlow;

#[derive(Serialize)]
pub struct AuthSyncPostUpdateReq(pub AuthenticationSyncPostUpdateRequest);

#[derive(Deserialize)]
pub struct AuthSyncResp(pub AuthenticationSyncResponse);

impl TryFrom<&(AuthenticationSyncPostUpdateRequest, String)> for AuthSyncPostUpdateReq {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(req: &(AuthenticationSyncPostUpdateRequest, String)) -> Result<Self, Self::Error> {
        Ok(Self(req.0.clone()))
    }
}

impl TryFrom<AuthSyncResp> for AuthenticationSyncResponse {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(resp: AuthSyncResp) -> Result<Self, Self::Error> {
        Ok(resp.0)
    }
}

impl_microservice_flow!(
    AuthenticationSyncPostUpdateFlow,
    method = Method::Post,
    path = "/authentication/{merchant_id}/{authentication_id}/redirect",
    v1_request = (AuthenticationSyncPostUpdateRequest, String), // the String is merchant_id
    v2_request = AuthSyncPostUpdateReq,
    v2_response = AuthSyncResp,
    v1_response = AuthenticationSyncResponse,
    client = AuthenticationServiceClient<'_>,
    body = |_, req: AuthSyncPostUpdateReq| Some(RequestContent::Json(Box::new(serde_json::json!(req.0)))),
    path_params = |_, req: &(AuthenticationSyncPostUpdateRequest, String)| vec![
        ("merchant_id", req.1.clone()),
        ("authentication_id", req.0.authentication_id.clone().get_string_repr().to_owned())
    ]
);

// ----------------------------------------------------------------------------
// Flow: authentication_sync
// ----------------------------------------------------------------------------
pub struct AuthenticationSyncFlow;

#[derive(Serialize)]
pub struct AuthSyncReq(pub AuthenticationSyncRequest);

impl TryFrom<&(AuthenticationSyncRequest, String)> for AuthSyncReq {
    type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
    fn try_from(req: &(AuthenticationSyncRequest, String)) -> Result<Self, Self::Error> {
        Ok(Self(req.0.clone()))
    }
}

impl_microservice_flow!(
    AuthenticationSyncFlow,
    method = Method::Post,
    path = "/authentication/{merchant_id}/{authentication_id}/sync",
    v1_request = (AuthenticationSyncRequest, String), // string is merchant_id
    v2_request = AuthSyncReq,
    v2_response = AuthSyncResp,
    v1_response = AuthenticationSyncResponse,
    client = AuthenticationServiceClient<'_>,
    body = |_, req: AuthSyncReq| Some(RequestContent::Json(Box::new(serde_json::json!(req.0)))),
    path_params = |_, req: &(AuthenticationSyncRequest, String)| vec![
        ("merchant_id", req.1.clone()),
        ("authentication_id", req.0.authentication_id.clone().get_string_repr().to_owned())
    ]
);
