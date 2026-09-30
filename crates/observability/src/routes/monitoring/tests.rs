//! Mock Router tests exercise the same authorization policy as both public endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use actix_web::{
    cookie::SameSite,
    http::{header, StatusCode},
    test, web, App, HttpRequest, HttpResponse, HttpServer,
};
use hyperswitch_masking::Secret;
use serde_json::{json, Value};

use super::*;
use crate::core::router_client::RouterClient;

async fn test_state(client: Option<&RouterClient>) -> AppState {
    use crate::{db::Store, domain::notifier::Registry, settings::Database};
    AppState {
        conf: Arc::new(serde_json::from_value(json!({"monitoring": {"destinations": {"payment_logs": "https://cc.example/api/observability-plane/grafana/explore?orgId=1&left=logs"}}})).unwrap()),
        router_transport: client.cloned().map(Arc::new),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        metrics: None,
        // No idle connections: monitoring never touches the database.
        store: Arc::new(
            Store::new(&Database {
                username: "unused".into(),
                host: "localhost".into(),
                dbname: "unused".into(),
                min_idle_pool_size: 0,
                ..Default::default()
            })
            .await
            .unwrap(),
        ),
    }
}

async fn session_with_client(client: Option<&RouterClient>, request: &HttpRequest) -> HttpResponse {
    let app = test::init_service(
        App::new().service(crate::routes::Monitoring::server(test_state(client).await)),
    )
    .await;
    let mut incoming = test::TestRequest::post().uri("/monitoring/grafana/session/payment_logs");
    for (name, value) in request.headers() {
        incoming = incoming.append_header((name.clone(), value.clone()));
    }
    test::call_service(&app, incoming.to_request())
        .await
        .into_parts()
        .1
        .map_into_boxed_body()
}

#[actix_web::test]
async fn auth_route_without_router_and_bad_json_fail_closed() {
    let app = test::init_service(
        App::new().service(crate::routes::Monitoring::server(test_state(None).await)),
    )
    .await;
    for (body, status) in [
        (r#"{"token":"signed.token.value"}"#, 503),
        (r#"{"token":""}"#, 503),
        ("not json", 400),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/monitoring/grafana/auth")
                .insert_header((header::CONTENT_TYPE, "application/json"))
                .set_payload(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status().as_u16(), status);
        assert!(response.headers().get(header::CACHE_CONTROL).is_none());
        if status != 400 {
            let body: Value = test::read_body_json(response).await;
            assert!(body.get("error").is_some());
        }
    }
}

#[actix_web::test]
async fn session_cookie_and_fail_closed_contract() {
    for (authorize_status, user_status, body, expected) in [
        (200, 200, r#"{"email":"user@example.com"}"#, 200),
        (401, 200, r#"{"email":"user@example.com"}"#, 401),
        (403, 200, r#"{"email":"user@example.com"}"#, 403),
        (500, 200, r#"{"email":"user@example.com"}"#, 503),
        (302, 200, r#"{"email":"user@example.com"}"#, 503),
        (200, 401, "", 401),
        (200, 500, "", 503),
        (200, 200, "not json", 503),
        (200, 200, r#"{"email":""}"#, 503),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(calls.clone()))
                .route(
                    "/user/internal/authorize",
                    web::post().to(move |payload: web::Json<Value>| async move {
                        assert_eq!(payload["token"], "signed.token.value");
                        assert_eq!(payload["permission"], "ProfileReconRuleRead");
                        HttpResponse::build(StatusCode::from_u16(authorize_status).unwrap())
                            .finish()
                    }),
                )
                .route(
                    "/user",
                    web::get().to(
                        move |request: HttpRequest, calls: web::Data<Arc<AtomicUsize>>| async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(
                                request.headers().get(header::AUTHORIZATION).unwrap(),
                                "Bearer signed.token.value"
                            );
                            assert!(request.headers().get(header::COOKIE).is_none());
                            assert!(request.headers().get("x-tenant-id").is_none());
                            HttpResponse::build(StatusCode::from_u16(user_status).unwrap())
                                .body(body)
                        },
                    ),
                )
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let client = RouterClient::new(
            url.parse().unwrap(),
            &hyperswitch_interfaces::types::Proxy::default(),
        )
        .unwrap();
        let request = test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, "Bearer signed.token.value"))
            .insert_header(("X-WEBAUTH-USER", "admin"))
            .insert_header(("x-tenant-id", "spoofed"))
            .to_http_request();
        let response = session_with_client(Some(&client), &request).await;
        assert_eq!(response.status().as_u16(), expected);
        assert!(response.headers().get(header::CACHE_CONTROL).is_none());
        if expected == 200 {
            let cookie = response.cookies().next().unwrap();
            assert_eq!(cookie.name(), "grafana_token");
            assert_eq!(cookie.value(), "signed.token.value");
            assert_eq!(cookie.path(), Some("/api/observability-plane/grafana"));
            assert_eq!(cookie.secure(), Some(true));
            assert_eq!(cookie.http_only(), Some(true));
            assert_eq!(cookie.same_site(), Some(SameSite::Strict));
            assert!(cookie.domain().is_none());
            assert!(cookie.max_age().is_none());
            assert!(cookie.expires().is_none());
            let body = actix_web::body::to_bytes(response.into_body())
                .await
                .unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                body,
                json!({"embed_url":"https://cc.example/api/observability-plane/grafana/explore?orgId=1&left=logs"})
            );
            let app = test::init_service(App::new().service(crate::routes::Monitoring::server(
                test_state(Some(&client)).await,
            )))
            .await;
            let unknown = test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/monitoring/grafana/session/unknown_id")
                    .insert_header((header::AUTHORIZATION, "Bearer signed.token.value"))
                    .to_request(),
            )
            .await;
            assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
            assert!(unknown.headers().get(header::SET_COOKIE).is_none());
            let body: Value = test::read_body_json(unknown).await;
            assert!(!body.to_string().contains("cc.example"));
            let identity = monitoring::authorize(
                test_state(Some(&client)).await,
                GrafanaAuthRequest {
                    token: Secret::new("signed.token.value".into()),
                },
            )
            .await
            .unwrap();
            assert_eq!(
                serde_json::to_value(identity).unwrap(),
                json!({"grafana_login": crate::domain::monitoring::GrafanaLogin::from_router_email("user@example.com").unwrap().into_string()})
            );
        } else {
            assert!(response.headers().get(header::SET_COOKIE).is_none());
        }
        if authorize_status != 200 {
            assert_eq!(observed.load(Ordering::SeqCst), 0);
        }
        handle.stop(false).await;
    }
}

#[actix_web::test]
async fn router_timeout_does_not_set_cookie() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = HttpServer::new(|| {
        App::new().default_service(web::to(|| async {
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
            HttpResponse::Ok().finish()
        }))
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run();
    let handle = server.handle();
    actix_web::rt::spawn(server);
    let client = RouterClient::new(
        url.parse().unwrap(),
        &hyperswitch_interfaces::types::Proxy::default(),
    )
    .unwrap();
    let request = test::TestRequest::post()
        .insert_header((header::AUTHORIZATION, "Bearer signed.token.value"))
        .to_http_request();
    let response = session_with_client(Some(&client), &request).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    handle.stop(false).await;
}

#[actix_web::test]
async fn missing_malformed_and_duplicate_headers_do_not_set_cookie() {
    for value in [None, Some("Basic abc"), Some("Bearer ")] {
        let mut request = test::TestRequest::post();
        if let Some(value) = value {
            request = request.insert_header((header::AUTHORIZATION, value));
        }
        let response = session_with_client(None, &request.to_http_request()).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }
    let request = test::TestRequest::post()
        .append_header((header::AUTHORIZATION, "Bearer one"))
        .append_header((header::AUTHORIZATION, "Bearer two"))
        .to_http_request();
    assert_eq!(
        session_with_client(None, &request).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let request = test::TestRequest::post()
        .insert_header((header::AUTHORIZATION, "Bearer valid.shape.token"))
        .to_http_request();
    assert_eq!(
        session_with_client(None, &request).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[actix_web::test]
async fn router_owns_token_validation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = HttpServer::new(move || {
        App::new().app_data(web::Data::new(calls.clone())).route(
            "/user/internal/authorize",
            web::post().to(
                |payload: web::Json<Value>, calls: web::Data<Arc<AtomicUsize>>| async move {
                    assert!(payload["token"].is_string());
                    calls.fetch_add(1, Ordering::SeqCst);
                    HttpResponse::Unauthorized().finish()
                },
            ),
        )
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run();
    let handle = server.handle();
    actix_web::rt::spawn(server);
    let client = RouterClient::new(
        url.parse().unwrap(),
        &hyperswitch_interfaces::types::Proxy::default(),
    )
    .unwrap();
    let app = test::init_service(App::new().service(crate::routes::Monitoring::server(
        test_state(Some(&client)).await,
    )))
    .await;
    for token in [String::new(), "a;b=c".to_owned(), "a".repeat(8193)] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/monitoring/grafana/auth")
                .set_json(json!({"token":token}))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }
    assert_eq!(observed.load(Ordering::SeqCst), 3);
    handle.stop(false).await;
}

#[actix_web::test]
async fn session_requires_post_and_a_destination_id() {
    let app = test::init_service(
        App::new().service(crate::routes::Monitoring::server(test_state(None).await)),
    )
    .await;
    let missing = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/monitoring/grafana/session")
            .to_request(),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let get = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/monitoring/grafana/session/payment_logs")
            .to_request(),
    )
    .await;
    assert_eq!(get.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(get.headers().get(header::SET_COOKIE).is_none());
    let unauthenticated = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/monitoring/grafana/session/unknown_id")
            .to_request(),
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}
