//! Mock Router tests exercise the same authorization policy as both public endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::core::router_client::RouterClient;
use actix_web::{test, web, App, HttpRequest, HttpResponse, HttpServer};
use serde_json::{json, Value};
use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[actix_web::test]
async fn session_cookie_and_fail_closed_contract() {
    for (authorize_status, user_status, body, expected) in [
        (200, 200, r#"{"email":"user@example.com"}"#, 204),
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
                            assert_eq!(
                                request.headers().get(header::COOKIE).unwrap(),
                                "login_token=signed.token.value"
                            );
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
        let client = RouterClient::new(url.parse().unwrap()).unwrap();
        let request = test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, "Bearer signed.token.value"))
            .insert_header(("X-WEBAUTH-USER", "admin"))
            .insert_header(("x-tenant-id", "spoofed"))
            .to_http_request();
        let response = session_with_client(Some(&client), &request).await;
        assert_eq!(response.status().as_u16(), expected);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        if expected == 204 {
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
            let identity = monitoring::authorize_with_client(
                Some(&client),
                &Secret::new("signed.token.value".into()),
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
    let client = RouterClient::new(url.parse().unwrap()).unwrap();
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
    for value in [
        None,
        Some("Basic abc"),
        Some("Bearer "),
        Some("Bearer a;b=c"),
    ] {
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
