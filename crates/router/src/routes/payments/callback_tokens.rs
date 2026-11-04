//! Registration and atomic consumption of single-use payment callback URLs.

use error_stack::ResultExt;
#[cfg(all(test, feature = "v1"))]
use hyperswitch_masking::PeekInterface;
use hyperswitch_masking::Secret;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::callback_routes::{
    ConnectorCallbackPath, ConnectorCallbackWithCredentialsPath, PaymentCallbackRoute,
    StartCallbackPath,
};
#[cfg(feature = "v1")]
use crate::services::authentication::{
    AuthenticateAndFetch, AuthenticationData, AuthenticationType, MerchantIdAuth,
};
use crate::{
    configs::settings::PaymentCallbackMode,
    core::errors::{self, RouterResult},
    routes::SessionState,
};
const UNUSED: &str = "unused";

/// Route identity plus its concrete, already-validated path parameters.
#[derive(Clone, Copy)]
pub(super) enum PaymentCallbackFields<'a> {
    Start(&'a StartCallbackPath),
    RedirectResponse(&'a ConnectorCallbackPath),
    RedirectResponseWithCredentials(&'a ConnectorCallbackWithCredentialsPath),
    RedirectComplete(&'a ConnectorCallbackPath),
    RedirectCompleteWithCredentials(&'a ConnectorCallbackWithCredentialsPath),
    Authorize(&'a ConnectorCallbackPath),
}

impl<'a> PaymentCallbackFields<'a> {
    pub(super) fn create_url(
        self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
    ) -> PaymentCallbackUrl {
        let route = self.route();
        let (path, key) = if callback_mode.issues_tokens() {
            let token = common_utils::generate_uuid_v4();
            let path = route.with_callback_token(&token.to_string());
            let key = self.key(&token);
            (path, Some(key))
        } else {
            (route.legacy_path(), None)
        };
        let path = match self {
            Self::Start(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::ATTEMPT_ID, &fields.attempt_id),
            Self::RedirectResponse(fields)
            | Self::RedirectComplete(fields)
            | Self::Authorize(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::CONNECTOR, &fields.connector),
            Self::RedirectResponseWithCredentials(fields)
            | Self::RedirectCompleteWithCredentials(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::CONNECTOR, &fields.connector)
                .replace(
                    PaymentCallbackRoute::CREDS_IDENTIFIER,
                    &fields.creds_identifier,
                ),
        };
        let path = format!("/payments{path}");
        PaymentCallbackUrl {
            key,
            url: Secret::new(format!("{base_url}{path}")),
        }
    }

    fn route(self) -> PaymentCallbackRoute {
        match self {
            Self::Start(_) => PaymentCallbackRoute::Start,
            Self::RedirectResponse(_) => PaymentCallbackRoute::RedirectResponse,
            Self::RedirectResponseWithCredentials(_) => {
                PaymentCallbackRoute::RedirectResponseWithCredentials
            }
            Self::RedirectComplete(_) => PaymentCallbackRoute::RedirectComplete,
            Self::RedirectCompleteWithCredentials(_) => {
                PaymentCallbackRoute::RedirectCompleteWithCredentials
            }
            Self::Authorize(_) => PaymentCallbackRoute::Authorize,
        }
    }

    fn key(self, token: &Uuid) -> redis_interface::RedisKey {
        let route = self.route();
        match self {
            Self::Start(path) => callback_key(route, path, token),
            Self::RedirectResponse(path) | Self::RedirectComplete(path) | Self::Authorize(path) => {
                callback_key(route, path, token)
            }
            Self::RedirectResponseWithCredentials(path)
            | Self::RedirectCompleteWithCredentials(path) => callback_key(route, path, token),
        }
    }

    #[cfg(feature = "v1")]
    fn merchant_id(self) -> &'a common_utils::id_type::MerchantId {
        match self {
            Self::Start(path) => &path.merchant_id,
            Self::RedirectResponse(path) | Self::RedirectComplete(path) | Self::Authorize(path) => {
                &path.merchant_id
            }
            Self::RedirectResponseWithCredentials(path)
            | Self::RedirectCompleteWithCredentials(path) => &path.merchant_id,
        }
    }
}

trait CallbackHashInput {
    fn hash_input(&self) -> String;

    // Byte-length framing distinguishes values even when they contain delimiters.
    fn concatenate_fields(fields: &[&str]) -> String {
        let mut input = String::new();
        for field in fields {
            input.push_str(&field.len().to_string());
            input.push(':');
            input.push_str(field);
        }
        input
    }
}

impl CallbackHashInput for StartCallbackPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.attempt_id,
        ])
    }
}

impl CallbackHashInput for ConnectorCallbackPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.connector,
        ])
    }
}

impl CallbackHashInput for ConnectorCallbackWithCredentialsPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.connector,
            &self.creds_identifier,
        ])
    }
}

/// A callback URL and its optional pending Redis registration.
#[derive(Clone)]
pub struct PaymentCallbackUrl {
    url: Secret<String>,
    key: Option<redis_interface::RedisKey>,
}

impl PaymentCallbackUrl {
    pub async fn persist(self, state: &SessionState) -> RouterResult<Secret<String>> {
        if let Some(key) = self.key {
            let ttl = state.conf.payment_callback.token_ttl_seconds;
            if ttl == 0 {
                return Err(errors::ApiErrorResponse::InternalServerError.into());
            }
            let connection = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let result = connection
                .set_key_if_not_exists_with_expiry(&key, UNUSED, Some(i64::from(ttl)))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            if !matches!(result, redis_interface::SetnxReply::KeySet) {
                return Err(errors::ApiErrorResponse::InternalServerError.into());
            }
        }
        Ok(self.url)
    }

    pub fn url(&self) -> &Secret<String> {
        &self.url
    }
}

// Hash the bearer credential so Redis keys and command instrumentation do not
// contain usable tokens. RedisConnectionWithContext applies the tenant prefix.
fn callback_key<T: CallbackHashInput>(
    route: PaymentCallbackRoute,
    fields: &T,
    token: &Uuid,
) -> redis_interface::RedisKey {
    let mut digest = Sha256::new();
    digest.update(T::concatenate_fields(&[route.name()]));
    digest.update(fields.hash_input());
    digest.update(token.as_bytes());
    format!("payment_callback_token:{:x}", digest.finalize()).into()
}

#[cfg(feature = "v1")]
fn parse_token(value: &str) -> RouterResult<Uuid> {
    let token = Uuid::parse_str(value).change_context(errors::ApiErrorResponse::Unauthorized)?;
    if token.get_version_num() != 4 || token.to_string() != value {
        return Err(errors::ApiErrorResponse::Unauthorized.into());
    }
    Ok(token)
}

#[cfg(feature = "v1")]
fn request_key(
    fields: PaymentCallbackFields<'_>,
    request: &actix_web::HttpRequest,
    mode: PaymentCallbackMode,
) -> RouterResult<Option<redis_interface::RedisKey>> {
    let route = fields.route();
    match request.match_name() {
        Some(name) if name == route.name() => (),
        Some(name) if name == route.legacy_name() && mode.allows_legacy() => return Ok(None),
        _ => return Err(errors::ApiErrorResponse::Unauthorized.into()),
    }
    let token = parse_token(
        request
            .match_info()
            .get("callback_token")
            .ok_or(errors::ApiErrorResponse::Unauthorized)?,
    )?;
    Ok(Some(fields.key(&token)))
}

/// Opt-in authentication for protected callback handlers. Consume the token
/// after merchant authentication succeeds and before server_wrap takes a lock.
/// Original URLs are accepted only in legacy and migration modes.
#[cfg(feature = "v1")]
pub struct PaymentCallbackAuth {
    merchant_auth: MerchantIdAuth,
    key: Option<redis_interface::RedisKey>,
}

#[cfg(feature = "v1")]
impl PaymentCallbackAuth {
    pub(super) fn from_request(
        fields: PaymentCallbackFields<'_>,
        request: &actix_web::HttpRequest,
        mode: PaymentCallbackMode,
    ) -> RouterResult<Self> {
        Ok(Self {
            merchant_auth: MerchantIdAuth(fields.merchant_id().clone()),
            key: request_key(fields, request, mode)?,
        })
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl AuthenticateAndFetch<AuthenticationData, SessionState> for PaymentCallbackAuth {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &actix_web::http::header::HeaderMap,
        state: &SessionState,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        let auth = self
            .merchant_auth
            .authenticate_and_fetch(request_headers, state)
            .await?;
        let Some(key) = self.key.as_ref() else {
            return Ok(auth);
        };
        let connection = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let value = connection
            .consume_key(key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        if value.as_deref() != Some(UNUSED) {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }
        Ok(auth)
    }
}

#[cfg(test)]
#[cfg(feature = "v1")]
mod tests {
    use actix_web::{
        http::Method, test as actix_test, web, App, HttpRequest, HttpResponse, Resource,
    };

    use super::*;

    const ROUTES: [PaymentCallbackRoute; 6] = [
        PaymentCallbackRoute::Start,
        PaymentCallbackRoute::RedirectResponse,
        PaymentCallbackRoute::RedirectResponseWithCredentials,
        PaymentCallbackRoute::RedirectComplete,
        PaymentCallbackRoute::RedirectCompleteWithCredentials,
        PaymentCallbackRoute::Authorize,
    ];
    const MODES: [PaymentCallbackMode; 3] = [
        PaymentCallbackMode::Legacy,
        PaymentCallbackMode::Migration,
        PaymentCallbackMode::Enforced,
    ];

    struct CallbackPaths {
        start: StartCallbackPath,
        connector: ConnectorCallbackPath,
        credentials: ConnectorCallbackWithCredentialsPath,
    }
    impl CallbackPaths {
        fn new() -> Self {
            let payment_id =
                common_utils::id_type::PaymentId::wrap("pay_1".to_string()).expect("payment ID");
            let merchant_id = common_utils::id_type::MerchantId::wrap("merchant_1".to_string())
                .expect("merchant ID");
            Self {
                start: StartCallbackPath {
                    payment_id: payment_id.clone(),
                    merchant_id: merchant_id.clone(),
                    attempt_id: "attempt_1".into(),
                },
                connector: ConnectorCallbackPath {
                    payment_id: payment_id.clone(),
                    merchant_id: merchant_id.clone(),
                    connector: "stripe".into(),
                },
                credentials: ConnectorCallbackWithCredentialsPath {
                    payment_id,
                    merchant_id,
                    connector: "stripe".into(),
                    creds_identifier: "creds_1".into(),
                },
            }
        }
        fn fields(&self, route: PaymentCallbackRoute) -> PaymentCallbackFields<'_> {
            match route {
                PaymentCallbackRoute::Start => PaymentCallbackFields::Start(&self.start),
                PaymentCallbackRoute::RedirectResponse => {
                    PaymentCallbackFields::RedirectResponse(&self.connector)
                }
                PaymentCallbackRoute::RedirectResponseWithCredentials => {
                    PaymentCallbackFields::RedirectResponseWithCredentials(&self.credentials)
                }
                PaymentCallbackRoute::RedirectComplete => {
                    PaymentCallbackFields::RedirectComplete(&self.connector)
                }
                PaymentCallbackRoute::RedirectCompleteWithCredentials => {
                    PaymentCallbackFields::RedirectCompleteWithCredentials(&self.credentials)
                }
                PaymentCallbackRoute::Authorize => {
                    PaymentCallbackFields::Authorize(&self.connector)
                }
                PaymentCallbackRoute::Authentication => {
                    panic!("Client-secret authentication does not use callback tokens")
                }
            }
        }
        fn url(
            &self,
            route: PaymentCallbackRoute,
            mode: PaymentCallbackMode,
        ) -> PaymentCallbackUrl {
            let base = "https://router.test";
            match route {
                PaymentCallbackRoute::Start => {
                    PaymentCallbackRoute::start_url(base, mode, &self.start)
                }
                PaymentCallbackRoute::RedirectResponse => {
                    PaymentCallbackRoute::redirect_response_url(base, mode, &self.connector)
                }
                PaymentCallbackRoute::RedirectResponseWithCredentials => {
                    PaymentCallbackRoute::redirect_response_with_credentials_url(
                        base,
                        mode,
                        &self.credentials,
                    )
                }
                PaymentCallbackRoute::RedirectComplete => {
                    PaymentCallbackRoute::redirect_complete_url(base, mode, &self.connector)
                }
                PaymentCallbackRoute::RedirectCompleteWithCredentials => {
                    PaymentCallbackRoute::redirect_complete_with_credentials_url(
                        base,
                        mode,
                        &self.credentials,
                    )
                }
                PaymentCallbackRoute::Authorize => {
                    PaymentCallbackRoute::authorize_url(base, mode, &self.connector)
                }
                PaymentCallbackRoute::Authentication => {
                    panic!("Client-secret authentication does not use callback tokens")
                }
            }
        }
    }
    fn resolved_path(route: PaymentCallbackRoute) -> String {
        route
            .legacy_path()
            .replace(PaymentCallbackRoute::PAYMENT_ID, "pay_1")
            .replace(PaymentCallbackRoute::MERCHANT_ID, "merchant_1")
            .replace(PaymentCallbackRoute::ATTEMPT_ID, "attempt_1")
            .replace(PaymentCallbackRoute::CONNECTOR, "stripe")
            .replace(PaymentCallbackRoute::CREDS_IDENTIFIER, "creds_1")
    }

    #[tokio::test]
    async fn issuance_obeys_rollout_mode_for_every_protected_route() {
        let paths = CallbackPaths::new();
        let mut seen = std::collections::HashSet::new();
        for route in ROUTES {
            for mode in MODES {
                let path = resolved_path(route);
                let pending = paths.url(route, mode);
                let url = url::Url::parse(pending.url().peek()).expect("URL");
                assert!(url.query().is_none());
                if mode == PaymentCallbackMode::Legacy {
                    assert_eq!(url.path(), format!("/payments{path}"));
                    assert!(pending.key.is_none());
                } else {
                    let token = parse_token(
                        url.path_segments()
                            .expect("segments")
                            .next_back()
                            .expect("token"),
                    )
                    .expect("UUID");
                    assert!(seen.insert(token));
                    assert_eq!(url.path(), format!("/payments{path}/callback/{token}"));
                    assert_eq!(
                        format!("{:?}", pending.key),
                        format!("{:?}", Some(paths.fields(route).key(&token)))
                    );
                }
            }
        }
    }

    #[test]
    fn callback_keys_bind_the_route_and_use_stable_field_encoding() {
        let paths = CallbackPaths::new();
        let token = Uuid::parse_str("d9dc53cc-d1a1-4f57-9077-d20c2d614daa").expect("UUID");
        let expected: redis_interface::RedisKey = "payment_callback_token:8e7e467a9e7bb32891e4adad4922ceeb8d2a623f01664737aa0416017ef01c42".into();
        assert_eq!(
            format!(
                "{:?}",
                paths.fields(PaymentCallbackRoute::Start).key(&token)
            ),
            format!("{expected:?}")
        );
        let keys = ROUTES
            .into_iter()
            .map(|route| format!("{:?}", paths.fields(route).key(&token)))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(keys.len(), ROUTES.len());
        assert_ne!(
            StartCallbackPath::concatenate_fields(&["a", "bc"]),
            StartCallbackPath::concatenate_fields(&["ab", "c"])
        );
    }

    #[test]
    fn malformed_and_noncanonical_tokens_are_rejected() {
        let token = common_utils::generate_uuid_v4();
        assert_eq!(parse_token(&token.to_string()).expect("UUID"), token);
        for value in [
            String::new(),
            "invalid".into(),
            Uuid::nil().to_string(),
            "D9DC53CC-D1A1-4F57-9077-D20C2D614DAA".into(),
            format!("{{{token}}}"),
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8".into(),
        ] {
            assert!(parse_token(&value).is_err());
        }
    }

    #[test]
    fn rollout_defaults_to_legacy_and_rejects_unknown_modes() {
        let config: crate::configs::settings::PaymentCallbackConfig =
            serde_json::from_str("{}").expect("defaults");
        assert_eq!(config.mode, PaymentCallbackMode::Legacy);
        assert_eq!(config.token_ttl_seconds, 3600);
        assert!(
            serde_json::from_str::<crate::configs::settings::PaymentCallbackConfig>(
                r#"{"mode":"unknown"}"#
            )
            .is_err()
        );
    }

    fn echo_auth(
        request: &HttpRequest,
        fields: PaymentCallbackFields<'_>,
        mode: PaymentCallbackMode,
    ) -> HttpResponse {
        match PaymentCallbackAuth::from_request(fields, request, mode) {
            Ok(auth) => {
                HttpResponse::Ok().body(format!("{:?}|{}", auth.key, request.query_string()))
            }
            Err(_) => HttpResponse::Unauthorized().finish(),
        }
    }
    async fn echo_start(
        request: HttpRequest,
        mode: web::Data<PaymentCallbackMode>,
        path: web::Path<StartCallbackPath>,
    ) -> HttpResponse {
        echo_auth(&request, PaymentCallbackFields::Start(&path), **mode)
    }
    async fn echo_connector(
        request: HttpRequest,
        route: web::Data<PaymentCallbackRoute>,
        mode: web::Data<PaymentCallbackMode>,
        path: web::Path<ConnectorCallbackPath>,
    ) -> HttpResponse {
        let fields = match **route {
            PaymentCallbackRoute::RedirectResponse => {
                PaymentCallbackFields::RedirectResponse(&path)
            }
            PaymentCallbackRoute::RedirectComplete => {
                PaymentCallbackFields::RedirectComplete(&path)
            }
            PaymentCallbackRoute::Authorize => PaymentCallbackFields::Authorize(&path),
            _ => panic!("Expected a connector callback route"),
        };
        echo_auth(&request, fields, **mode)
    }
    async fn echo_credentials(
        request: HttpRequest,
        route: web::Data<PaymentCallbackRoute>,
        mode: web::Data<PaymentCallbackMode>,
        path: web::Path<ConnectorCallbackWithCredentialsPath>,
    ) -> HttpResponse {
        let fields = match **route {
            PaymentCallbackRoute::RedirectResponseWithCredentials => {
                PaymentCallbackFields::RedirectResponseWithCredentials(&path)
            }
            PaymentCallbackRoute::RedirectCompleteWithCredentials => {
                PaymentCallbackFields::RedirectCompleteWithCredentials(&path)
            }
            _ => panic!("Expected a connector callback route with credentials"),
        };
        echo_auth(&request, fields, **mode)
    }
    fn test_resource(route: PaymentCallbackRoute, legacy: bool) -> Resource {
        let resource = if legacy {
            web::resource(route.legacy_path()).name(&route.legacy_name())
        } else {
            web::resource(route.path()).name(route.name())
        }
        .app_data(web::Data::new(route));
        match route {
            PaymentCallbackRoute::Start => resource
                .route(web::get().to(echo_start))
                .route(web::post().to(echo_start)),
            PaymentCallbackRoute::RedirectResponseWithCredentials
            | PaymentCallbackRoute::RedirectCompleteWithCredentials => resource
                .route(web::get().to(echo_credentials))
                .route(web::post().to(echo_credentials)),
            _ => resource
                .route(web::get().to(echo_connector))
                .route(web::post().to(echo_connector)),
        }
    }

    #[actix_web::test]
    async fn callback_routes_accept_legacy_only_during_rollout_and_always_validate_path_tokens() {
        let paths = CallbackPaths::new();
        let query = "connector_param=a%2fb&connector_param=c+d&flag&empty=";
        for mode in MODES {
            for route in ROUTES {
                let path = resolved_path(route);
                let pending = paths.url(route, PaymentCallbackMode::Migration);
                let url = url::Url::parse(pending.url().peek()).expect("URL");
                let uri = format!("{}?{query}", url.path());
                let expected = format!("{:?}|{query}", pending.key);
                let app = actix_test::init_service(
                    App::new().app_data(web::Data::new(mode)).service(
                        web::scope("/payments")
                            .service(test_resource(route, false))
                            .service(test_resource(route, true)),
                    ),
                )
                .await;
                for method in [Method::GET, Method::POST] {
                    let response = actix_test::call_service(
                        &app,
                        actix_test::TestRequest::default()
                            .method(method.clone())
                            .uri(&uri)
                            .to_request(),
                    )
                    .await;
                    assert_eq!(response.status(), actix_web::http::StatusCode::OK);
                    assert_eq!(
                        std::str::from_utf8(&actix_test::read_body(response).await)
                            .expect("key and query"),
                        expected
                    );
                    let legacy = actix_test::call_service(
                        &app,
                        actix_test::TestRequest::default()
                            .method(method)
                            .uri(&format!("/payments{path}?{query}"))
                            .to_request(),
                    )
                    .await;
                    if mode != PaymentCallbackMode::Enforced {
                        assert_eq!(legacy.status(), actix_web::http::StatusCode::OK);
                        assert_eq!(
                            std::str::from_utf8(&actix_test::read_body(legacy).await)
                                .expect("legacy query"),
                            format!("None|{query}")
                        );
                    } else {
                        assert_eq!(legacy.status(), actix_web::http::StatusCode::UNAUTHORIZED);
                    }
                }
                for (old, new) in [
                    ("pay_1", "pay_2"),
                    ("merchant_1", "merchant_2"),
                    ("attempt_1", "attempt_2"),
                    ("stripe", "adyen"),
                    ("creds_1", "creds_2"),
                ] {
                    let changed = uri.replace(old, new);
                    if changed == uri {
                        continue;
                    }
                    let body = actix_test::call_and_read_body(
                        &app,
                        actix_test::TestRequest::get().uri(&changed).to_request(),
                    )
                    .await;
                    assert_ne!(std::str::from_utf8(&body).expect("changed key"), expected);
                }
                for token in ["invalid".to_string(), Uuid::nil().to_string()] {
                    let response = actix_test::call_service(
                        &app,
                        actix_test::TestRequest::get()
                            .uri(&format!("/payments{path}/callback/{token}?{query}"))
                            .to_request(),
                    )
                    .await;
                    assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    #[actix_web::test]
    async fn callback_auth_rejects_unmatched_and_unprotected_routes() {
        let paths = CallbackPaths::new();
        let request = actix_test::TestRequest::get()
            .uri("/other")
            .to_http_request();
        for mode in MODES {
            assert!(
                request_key(PaymentCallbackFields::Start(&paths.start), &request, mode).is_err()
            );
        }
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(PaymentCallbackMode::Migration))
                .service(
                    web::resource(PaymentCallbackRoute::Start.path())
                        .name(PaymentCallbackRoute::Authentication.name())
                        .route(web::get().to(echo_start)),
                ),
        )
        .await;
        let pending = paths.url(PaymentCallbackRoute::Start, PaymentCallbackMode::Migration);
        let url = url::Url::parse(pending.url().peek()).expect("URL");
        // The test resource is mounted without the /payments scope.
        let response = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(url.path().strip_prefix("/payments").expect("scope"))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
}
