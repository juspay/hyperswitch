use actix_web::{http::Method, test as actix_test, web, App, HttpRequest, HttpResponse, Resource};

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
        let merchant_id =
            common_utils::id_type::MerchantId::wrap("merchant_1".to_string()).expect("merchant ID");
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
            PaymentCallbackRoute::Authorize => PaymentCallbackFields::Authorize(&self.connector),
        }
    }
    fn url(&self, route: PaymentCallbackRoute, mode: PaymentCallbackMode) -> PaymentCallbackUrl {
        let base = "https://router.test";
        match route {
            PaymentCallbackRoute::Start => PaymentCallbackRoute::start_url(base, mode, &self.start),
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

fn resolved_token_path(route: PaymentCallbackRoute, token: &str) -> String {
    let prefix = format!("/payments/pay_1/merchant_1/redirect/token/{token}");
    match route {
        PaymentCallbackRoute::Start => format!("{prefix}/start/attempt_1"),
        PaymentCallbackRoute::RedirectResponse => format!("{prefix}/response/stripe"),
        PaymentCallbackRoute::RedirectResponseWithCredentials => {
            format!("{prefix}/response/stripe/creds_1")
        }
        PaymentCallbackRoute::RedirectComplete => format!("{prefix}/complete/stripe"),
        PaymentCallbackRoute::RedirectCompleteWithCredentials => {
            format!("{prefix}/complete/stripe/creds_1")
        }
        PaymentCallbackRoute::Authorize => format!("{prefix}/authorize/stripe"),
    }
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
                let token = PaymentCallbackAuth::parse_token(
                    url.path_segments()
                        .expect("segments")
                        .nth(5)
                        .expect("token"),
                )
                .expect("UUID");
                assert!(seen.insert(token));
                assert_eq!(url.path(), resolved_token_path(route, &token.to_string()));
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
    let expected: redis_interface::RedisKey =
        "payment_callback_token:8e7e467a9e7bb32891e4adad4922ceeb8d2a623f01664737aa0416017ef01c42"
            .into();
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
    assert_eq!(
        PaymentCallbackAuth::parse_token(&token.to_string()).expect("UUID"),
        token
    );
    for value in [
        String::new(),
        "invalid".into(),
        Uuid::nil().to_string(),
        "D9DC53CC-D1A1-4F57-9077-D20C2D614DAA".into(),
        format!("{{{token}}}"),
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8".into(),
    ] {
        assert!(PaymentCallbackAuth::parse_token(&value).is_err());
    }
}

fn echo_auth(
    request: &HttpRequest,
    fields: PaymentCallbackFields<'_>,
    mode: PaymentCallbackMode,
) -> HttpResponse {
    match PaymentCallbackAuth::from_request(fields, request).and_then(|auth| {
        auth.validate_mode(mode)?;
        Ok(auth)
    }) {
        Ok(auth) => HttpResponse::Ok().body(format!("{:?}|{}", auth.key, request.query_string())),
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
        PaymentCallbackRoute::RedirectResponse => PaymentCallbackFields::RedirectResponse(&path),
        PaymentCallbackRoute::RedirectComplete => PaymentCallbackFields::RedirectComplete(&path),
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
                        .uri(&format!("{}?{query}", resolved_token_path(route, &token)))
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
    assert!(
        PaymentCallbackAuth::request_key(PaymentCallbackFields::Start(&paths.start), &request)
            .is_err()
    );
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(PaymentCallbackMode::Migration))
            .service(
                web::resource(PaymentCallbackRoute::Start.path())
                    .name("unprotected_route")
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
