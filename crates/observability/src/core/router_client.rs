//! The two Router calls behind Grafana gateway authentication.
//!
//! Both calls use exactly the credential presented to this endpoint. Router alone decides whether
//! the token is valid and has the fixed permission; its user endpoint supplies the active user's
//! email. This module never inspects JWT claims or forwards caller-controlled identity headers.

use std::time::Duration;

use common_enums::ApiClientError;
use common_utils::{
    id_type::TenantId,
    request::{Headers, Method, Request, RequestContent},
};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::{
    api_client::{ApiClient, ApiClientWrapper, RequestBuilder},
    configs::{Connectors, Tenant, TenantUserConfig},
    events::{connector_api_logs::ConnectorEvent, EventHandlerInterface},
    micro_service::{
        execute_microservice_operation, ClientOperation, MicroserviceClient,
        MicroserviceClientError, MicroserviceClientErrorKind,
    },
    types::Proxy,
};
use hyperswitch_masking::{Maskable, PeekInterface, Secret};
use router_env::{RequestId, RequestIdentifier};
use serde::{Deserialize, Serialize};
use url::Url;

/// Only this permission is accepted; the incoming body cannot select one.
pub const GRAFANA_PERMISSION: &str = "ProfileReconRuleRead";
const ROUTER_TIMEOUT: Duration = Duration::from_secs(3);

pub struct RouterClient {
    base_url: Url,
    headers: Headers,
    trace: RequestIdentifier,
}

impl RouterClient {
    pub fn new(base_url: &str, token: &Secret<String>) -> Result<Self, MicroserviceClientError> {
        let base_url = Url::parse(base_url).map_err(|_| client_error("invalid Router URL"))?;
        let mut headers = Headers::new();
        headers.insert((
            "Authorization".to_owned(),
            Maskable::new_masked(Secret::new(format!("Bearer {}", token.peek()))),
        ));
        // Router may be configured with force_cookies=true. Both sources carry the same token;
        // never forward an Authorization header or Cookie supplied separately by the caller.
        headers.insert((
            "Cookie".to_owned(),
            Maskable::new_masked(Secret::new(format!("login_token={}", token.peek()))),
        ));
        Ok(Self {
            base_url,
            headers,
            trace: RequestIdentifier::new("x-request-id"),
        })
    }

    pub async fn authorize_and_get_email(
        &self,
        transport: &RouterCallState,
        token: Secret<String>,
    ) -> Result<String, MicroserviceClientError> {
        execute_microservice_operation::<AuthorizeFlow>(
            transport,
            self,
            AuthorizeRequest {
                token,
                permission: GRAFANA_PERMISSION,
            },
        )
        .await?;
        let details = execute_microservice_operation::<UserFlow>(transport, self, ()).await?;
        if details.email.is_empty() || details.email.len() > 320 {
            return Err(client_error("Router returned unusable user details"));
        }
        Ok(details.email)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        net::TcpListener,
        sync::atomic::{AtomicBool, Ordering},
    };

    use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn empty_router_status_is_accepted_without_relaxing_other_flows() {
        assert!(AuthorizeFlow::decode_success(b"").is_ok());
        assert!(UserFlow::decode_success(b"").is_err());
        assert!(AuthorizeFlow::redact_error_body());
        assert!(UserFlow::redact_error_body());
    }

    #[actix_web::test]
    async fn router_validates_same_token_then_returns_email() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let checked = std::sync::Arc::new(AtomicBool::new(false));
        let flag = checked.clone();
        let server = HttpServer::new(move || {
            let flag = flag.clone();
            App::new().app_data(web::Data::new(flag))
                .route("/user/internal/authorize", web::post().to(
                    |req: HttpRequest, body: web::Json<Value>, checked: web::Data<std::sync::Arc<AtomicBool>>| async move {
                        assert_eq!(body["token"], "signed.token.value");
                        assert_eq!(body["permission"], GRAFANA_PERMISSION);
                        assert_eq!(req.headers().get("authorization").unwrap(), "Bearer signed.token.value");
                        checked.store(true, Ordering::SeqCst);
                        HttpResponse::Ok().finish()
                    }))
                .route("/user", web::get().to(
                    |req: HttpRequest, checked: web::Data<std::sync::Arc<AtomicBool>>| async move {
                        assert!(checked.load(Ordering::SeqCst));
                        assert_eq!(req.headers().get("authorization").unwrap(), "Bearer signed.token.value");
                        assert_eq!(req.headers().get("cookie").unwrap(), "login_token=signed.token.value");
                        HttpResponse::Ok().json(json!({"email": "user@example.com"}))
                    }))
        }).listen(listener).unwrap().run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let token = Secret::new("signed.token.value".to_owned());
        let client = RouterClient::new(&url, &token).unwrap();
        let transport = RouterCallState::new().unwrap();
        assert_eq!(
            client
                .authorize_and_get_email(&transport, token)
                .await
                .unwrap(),
            "user@example.com"
        );
        assert!(checked.load(Ordering::SeqCst));
        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn malformed_user_response_is_not_an_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = HttpServer::new(|| {
            App::new()
                .route(
                    "/user/internal/authorize",
                    web::post().to(|| async { HttpResponse::Ok().finish() }),
                )
                .route(
                    "/user",
                    web::get()
                        .to(|| async { HttpResponse::Ok().json(json!({"user_id": "spoofed"})) }),
                )
        })
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let token = Secret::new("signed.token.value".to_owned());
        let client = RouterClient::new(&url, &token).unwrap();
        let error = client
            .authorize_and_get_email(&RouterCallState::new().unwrap(), token)
            .await
            .unwrap_err();
        assert!(matches!(
            error.kind,
            MicroserviceClientErrorKind::Deserialize(_)
        ));
        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn user_lookup_failure_denies_after_authorization() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = HttpServer::new(|| {
            App::new()
                .route(
                    "/user/internal/authorize",
                    web::post().to(|| async { HttpResponse::Ok().finish() }),
                )
                .route(
                    "/user",
                    web::get().to(|| async {
                        HttpResponse::InternalServerError().body("internal details")
                    }),
                )
        })
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let token = Secret::new("signed.token.value".to_owned());
        let client = RouterClient::new(&url, &token).unwrap();
        let error = client
            .authorize_and_get_email(&RouterCallState::new().unwrap(), token)
            .await
            .unwrap_err();
        assert!(
            matches!(error.kind, MicroserviceClientErrorKind::Upstream { status: 500, ref body } if body.is_empty())
        );
        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn denied_authorization_never_fetches_user() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = HttpServer::new(|| {
            App::new()
                .route(
                    "/user/internal/authorize",
                    web::post().to(|| async { HttpResponse::Forbidden().finish() }),
                )
                .route(
                    "/user",
                    web::get().to(|| async { HttpResponse::InternalServerError().finish() }),
                )
        })
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let token = Secret::new("signed.token.value".to_owned());
        let client = RouterClient::new(&url, &token).unwrap();
        let error = client
            .authorize_and_get_email(&RouterCallState::new().unwrap(), token)
            .await
            .unwrap_err();
        assert!(matches!(
            error.kind,
            MicroserviceClientErrorKind::Upstream { status: 403, .. }
        ));
        handle.stop(true).await;
    }
}

impl MicroserviceClient for RouterClient {
    fn base_url(&self) -> &Url {
        &self.base_url
    }
    fn parent_headers(&self) -> &Headers {
        &self.headers
    }
    fn trace(&self) -> &RequestIdentifier {
        &self.trace
    }
    // The browser cannot choose a tenant header. Router resolves its own configured tenant and
    // GET /user compares that tenant with the verified session token.
    fn should_forward_tenant_header(&self) -> bool {
        false
    }
}

fn client_error(message: &str) -> MicroserviceClientError {
    MicroserviceClientError {
        operation: "grafana_router_auth".to_owned(),
        kind: MicroserviceClientErrorKind::Transport(message.to_owned()),
    }
}

#[derive(Clone, Serialize)]
struct AuthorizeRequest {
    token: Secret<String>,
    permission: &'static str,
}

struct AuthorizeFlow;

#[async_trait::async_trait]
impl ClientOperation for AuthorizeFlow {
    const METHOD: Method = Method::Post;
    const PATH_TEMPLATE: &'static str = "/user/internal/authorize";
    type V1Request = AuthorizeRequest;
    type V1Response = ();
    type V2Request = AuthorizeRequest;
    type V2Response = ();

    fn validate(&self, _: &Self::V1Request) -> Result<(), MicroserviceClientError> {
        Ok(())
    }
    fn from_request(_: &Self::V1Request) -> Self {
        Self
    }
    fn transform_request(
        &self,
        req: &Self::V1Request,
    ) -> Result<Self::V2Request, MicroserviceClientError> {
        Ok(req.clone())
    }
    fn transform_response(
        &self,
        _: Self::V2Response,
    ) -> Result<Self::V1Response, MicroserviceClientError> {
        Ok(())
    }
    fn body(&self, req: Self::V2Request) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(req)))
    }
    fn decode_success(bytes: &[u8]) -> Result<Self::V2Response, serde_json::Error> {
        // Router's StatusOk renders as HTTP 200 with zero bytes; no JSON is sent.
        if bytes.is_empty() {
            serde_json::from_slice(b"null")
        } else {
            serde_json::from_slice(bytes)
        }
    }
    fn redact_error_body() -> bool {
        true
    }
}

#[derive(Deserialize)]
struct UserDetails {
    email: String,
}
struct UserFlow;

#[async_trait::async_trait]
impl ClientOperation for UserFlow {
    const METHOD: Method = Method::Get;
    const PATH_TEMPLATE: &'static str = "/user";
    type V1Request = ();
    type V1Response = UserDetails;
    type V2Request = ();
    type V2Response = UserDetails;

    fn validate(&self, _: &()) -> Result<(), MicroserviceClientError> {
        Ok(())
    }
    fn from_request(_: &()) -> Self {
        Self
    }
    fn transform_request(&self, _: &()) -> Result<(), MicroserviceClientError> {
        Ok(())
    }
    fn transform_response(
        &self,
        response: Self::V2Response,
    ) -> Result<Self::V1Response, MicroserviceClientError> {
        Ok(response)
    }
    fn redact_error_body() -> bool {
        true
    }
}

// The generic microservice executor currently requires an ApiClientWrapper with connector and
// event methods, even for an internal HTTP call. This minimal adapter gives it an isolated,
// no-redirect, no-proxy transport; no connector event or browser header is forwarded.
pub struct RouterCallState {
    client: InternalHttpClient,
    events: NoEvents,
}

impl RouterCallState {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: InternalHttpClient {
                client: reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(ROUTER_TIMEOUT)
                    .build()?,
            },
            events: NoEvents,
        })
    }
}

impl ApiClientWrapper for RouterCallState {
    fn get_api_client(&self) -> &dyn ApiClient {
        &self.client
    }
    fn get_proxy(&self) -> Proxy {
        Proxy::default()
    }
    fn get_request_id_str(&self) -> Option<String> {
        None
    }
    fn get_request_id(&self) -> Option<RequestId> {
        None
    }
    fn get_tenant(&self) -> Tenant {
        // The flow disables tenant-header forwarding, so this is never consulted by the executor.
        Tenant {
            tenant_id: TenantId::get_default_tenant_id(),
            base_url: String::new(),
            schema: String::new(),
            accounts_schema: String::new(),
            redis_key_prefix: String::new(),
            clickhouse_database: String::new(),
            user: TenantUserConfig {
                control_center_url: String::new(),
            },
        }
    }
    fn get_connectors(&self) -> Connectors {
        Connectors::default()
    }
    fn event_handler(&self) -> &dyn EventHandlerInterface {
        &self.events
    }
}

#[derive(Clone)]
struct NoEvents;
impl EventHandlerInterface for NoEvents {
    fn log_connector_event(&self, _: &ConnectorEvent) {}
}

#[derive(Clone)]
struct InternalHttpClient {
    client: reqwest::Client,
}

#[async_trait::async_trait]
impl ApiClient for InternalHttpClient {
    fn request(
        &self,
        _: reqwest::Method,
        _: String,
    ) -> error_stack::Result<Box<dyn RequestBuilder>, ApiClientError> {
        Err(report!(ApiClientError::UnexpectedState))
    }
    fn request_with_certificate(
        &self,
        _: reqwest::Method,
        _: String,
        _: Option<Secret<String>>,
        _: Option<Secret<String>>,
    ) -> error_stack::Result<Box<dyn RequestBuilder>, ApiClientError> {
        Err(report!(ApiClientError::UnexpectedState))
    }
    async fn send_request(
        &self,
        _: &dyn ApiClientWrapper,
        request: Request,
        _: Option<u64>,
        _: bool,
    ) -> error_stack::Result<reqwest::Response, ApiClientError> {
        let method = match request.method {
            Method::Post => reqwest::Method::POST,
            Method::Get => reqwest::Method::GET,
            _ => return Err(report!(ApiClientError::UnexpectedState)),
        };
        let mut builder = self.client.request(method, &request.url);
        for (name, value) in request.headers {
            let value = match value {
                Maskable::Masked(secret) => secret.peek().clone(),
                Maskable::Normal(value) => value,
            };
            let mut header_value = reqwest::header::HeaderValue::from_str(&value)
                .change_context(ApiClientError::HeaderMapConstructionFailed)?;
            header_value.set_sensitive(true);
            builder = builder.header(name, header_value);
        }
        if let Some(body) = request.body {
            match body {
                RequestContent::Json(payload) => {
                    builder = builder.json(&payload);
                }
                _ => return Err(report!(ApiClientError::UnexpectedState)),
            }
        }
        builder.send().await.map_err(|_| {
            report!(ApiClientError::RequestNotSent(
                "Router request failed".to_owned()
            ))
        })
    }
    fn add_request_id(&mut self, _: RequestId) {}
    fn get_request_id(&self) -> Option<RequestId> {
        None
    }
    fn get_request_id_str(&self) -> Option<String> {
        None
    }
    fn add_flow_name(&mut self, _: String) {}
}
