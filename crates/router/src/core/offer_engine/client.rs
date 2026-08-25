use base64::Engine;
use common_utils::{
    errors::CustomResult,
    request::{Headers, Method, RequestContent},
};
use hyperswitch_interfaces::micro_service::{
    MicroserviceClient, MicroserviceClientError, MicroserviceClientErrorKind,
};
use hyperswitch_masking::{Mask, PeekInterface};
use router_env::{IdReuse, RequestIdentifier};

use super::types::{
    OfferApplyRequest, OfferApplyResponse, OfferEngineError, OfferListRequest, OfferListResponse,
    ResolvedOfferEngineConfig,
};
use crate::{consts::BASE64_ENGINE, routes::SessionState};

const OFFERS_LIST_PATH: &str = "v1/offers/list";
const OFFERS_APPLY_PATH: &str = "offers/apply";

#[derive(Debug)]
pub struct OfferEngineClient {
    base_url: url::Url,
    parent_headers: Headers,
    trace: RequestIdentifier,
    merchant_id: String,
}

impl OfferEngineClient {
    pub fn new(config: ResolvedOfferEngineConfig, trace_header_name: &str) -> Self {
        let auth_value = format!(
            "Basic {}",
            BASE64_ENGINE.encode(format!("{}:", config.api_key.peek()))
        );
        let mut parent_headers = Headers::new();
        parent_headers.insert(("Authorization".to_string(), auth_value.into_masked()));

        let trace = RequestIdentifier::new(trace_header_name).use_incoming_id(IdReuse::UseIncoming);

        Self {
            base_url: config.base_url,
            parent_headers,
            trace,
            merchant_id: config.merchant_id,
        }
    }

    pub fn merchant_id(&self) -> &str {
        &self.merchant_id
    }

    pub async fn list_offers(
        &self,
        state: &SessionState,
        request: OfferListRequest,
    ) -> CustomResult<OfferListResponse, OfferEngineError> {
        OfferList::call(state, self, OfferListV1Request(request))
            .await
            .map(|response| response.0)
            .map_err(map_offer_engine_error)
    }

    pub async fn apply_offer(
        &self,
        state: &SessionState,
        request: OfferApplyRequest,
    ) -> CustomResult<OfferApplyResponse, OfferEngineError> {
        OfferApply::call(state, self, OfferApplyV1Request(request))
            .await
            .map(|response| response.0)
            .map_err(map_offer_engine_error)
    }

    pub async fn check_connectivity(&self, state: &SessionState) -> ConnectivityResult {
        match OfferConnectivity::call(state, self, OfferConnectivityRequest).await {
            Ok(_) => ConnectivityResult {
                reachable: true,
                status_code: None,
                detail: "Reached Offer Engine".to_string(),
            },
            Err(err) => match err.kind {
                MicroserviceClientErrorKind::Upstream { status, .. } => {
                    let detail = match status {
                        401 | 403 => "Reached Offer Engine, but authentication failed".to_string(),
                        _ => format!("Reached Offer Engine (HTTP {status})"),
                    };
                    ConnectivityResult {
                        reachable: true,
                        status_code: Some(status),
                        detail,
                    }
                }
                MicroserviceClientErrorKind::Deserialize(_) => ConnectivityResult {
                    reachable: true,
                    status_code: None,
                    detail: "Reached Offer Engine".to_string(),
                },
                other => ConnectivityResult {
                    reachable: false,
                    status_code: None,
                    detail: format!("Could not reach Offer Engine: {other}"),
                },
            },
        }
    }
}

impl MicroserviceClient for OfferEngineClient {
    fn base_url(&self) -> &url::Url {
        &self.base_url
    }

    fn parent_headers(&self) -> &Headers {
        &self.parent_headers
    }

    fn trace(&self) -> &RequestIdentifier {
        &self.trace
    }

    // Offer Engine is an external third-party service that resolves the tenant
    // from the API key, not hyperswitch's `x-tenant-id`. Forwarding our tenant
    // header makes it reject the request ("Merchant tenantAccId not present").
    fn should_forward_tenant_header(&self) -> bool {
        false
    }
}

fn map_offer_engine_error(err: MicroserviceClientError) -> error_stack::Report<OfferEngineError> {
    let variant = match err.kind {
        MicroserviceClientErrorKind::Deserialize(_) => OfferEngineError::ResponseParseFailed,
        _ => OfferEngineError::RequestFailed,
    };
    error_stack::report!(variant).attach_printable(format!("Offer Engine call failed: {err:?}"))
}

pub struct OfferConnectivity;
pub struct OfferConnectivityRequest;
pub struct OfferConnectivityResponse;

impl TryFrom<&Self> for OfferConnectivityRequest {
    type Error = MicroserviceClientError;
    fn try_from(_: &Self) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryFrom<serde_json::Value> for OfferConnectivityResponse {
    type Error = MicroserviceClientError;
    fn try_from(_: serde_json::Value) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    OfferConnectivity,
    method = Method::Post,
    path = OFFERS_LIST_PATH,
    v1_request = OfferConnectivityRequest,
    v2_request = OfferConnectivityRequest,
    v2_response = serde_json::Value,
    v1_response = OfferConnectivityResponse,
    client = OfferEngineClient,
    body = |_: &OfferConnectivity, _: OfferConnectivityRequest| Some(RequestContent::Json(
        Box::new(serde_json::json!({}))
    ))
);

pub struct OfferList;
pub struct OfferListV1Request(pub OfferListRequest);
pub struct OfferListV1Response(pub OfferListResponse);

impl TryFrom<&OfferListV1Request> for OfferListRequest {
    type Error = MicroserviceClientError;
    fn try_from(value: &OfferListV1Request) -> Result<Self, Self::Error> {
        Ok(value.0.clone())
    }
}

impl TryFrom<OfferListResponse> for OfferListV1Response {
    type Error = MicroserviceClientError;
    fn try_from(value: OfferListResponse) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl OfferList {
    fn build_body(&self, request: OfferListRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    OfferList,
    method = Method::Post,
    path = OFFERS_LIST_PATH,
    v1_request = OfferListV1Request,
    v2_request = OfferListRequest,
    v2_response = OfferListResponse,
    v1_response = OfferListV1Response,
    client = OfferEngineClient,
    body = OfferList::build_body
);

pub struct OfferApply;
pub struct OfferApplyV1Request(pub OfferApplyRequest);
pub struct OfferApplyV1Response(pub OfferApplyResponse);

impl TryFrom<&OfferApplyV1Request> for OfferApplyRequest {
    type Error = MicroserviceClientError;
    fn try_from(value: &OfferApplyV1Request) -> Result<Self, Self::Error> {
        Ok(value.0.clone())
    }
}

impl TryFrom<OfferApplyResponse> for OfferApplyV1Response {
    type Error = MicroserviceClientError;
    fn try_from(value: OfferApplyResponse) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl OfferApply {
    fn build_body(&self, request: OfferApplyRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    OfferApply,
    method = Method::Post,
    path = OFFERS_APPLY_PATH,
    v1_request = OfferApplyV1Request,
    v2_request = OfferApplyRequest,
    v2_response = OfferApplyResponse,
    v1_response = OfferApplyV1Response,
    client = OfferEngineClient,
    body = OfferApply::build_body
);

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectivityResult {
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub detail: String,
}
