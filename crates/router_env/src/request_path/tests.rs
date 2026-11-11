use actix_web::{test, web, App, HttpResponse};

use super::*;

async fn return_telemetry_path(request: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body(path_for_telemetry(&request).into_owned())
}

#[actix_web::test]
async fn callback_routes_hide_only_the_token() {
    const TOKEN: &str = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa";
    let cases = [
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/start/{attempt_id}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/start/attempt_789",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/response/{connector}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/response/stripe",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/response/{connector}/{creds_identifier}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/response/stripe/creds_012",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/complete/{connector}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/complete/stripe",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/complete/{connector}/{creds_identifier}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/complete/stripe/creds_012",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/token/{callback_token}/authorize/{connector}",
                "/payments/pay_123/merchant_456/redirect/token/{callback_token}/authorize/stripe",
            ),
            (
                "/payments/redirect/{payment_id}/{merchant_id}/{attempt_id}/callback/{callback_token}",
                "/payments/redirect/pay_123/merchant_456/attempt_789/callback/{callback_token}",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/response/{connector}/callback/{callback_token}",
                "/payments/pay_123/merchant_456/redirect/response/stripe/callback/{callback_token}",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/response/{connector}/{creds_identifier}/callback/{callback_token}",
                "/payments/pay_123/merchant_456/redirect/response/stripe/creds_012/callback/{callback_token}",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/complete/{connector}/callback/{callback_token}",
                "/payments/pay_123/merchant_456/redirect/complete/stripe/callback/{callback_token}",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/redirect/complete/{connector}/{creds_identifier}/callback/{callback_token}",
                "/payments/pay_123/merchant_456/redirect/complete/stripe/creds_012/callback/{callback_token}",
            ),
            (
                "/payments/{payment_id}/{merchant_id}/authorize/{connector}/callback/{callback_token}",
                "/payments/pay_123/merchant_456/authorize/stripe/callback/{callback_token}",
            ),
        ];
    let app = test::init_service(App::new().configure(|config| {
        for (pattern, _) in cases {
            config.service(web::resource(pattern).route(web::get().to(return_telemetry_path)));
        }
    }))
    .await;

    for (_, expected) in cases {
        let request = test::TestRequest::get()
            .uri(&expected.replace("{callback_token}", TOKEN))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert!(response.status().is_success(), "{expected}");
        let body = test::read_body(response).await;
        assert_eq!(
            std::str::from_utf8(&body).expect("telemetry path"),
            expected
        );
    }
}

#[actix_web::test]
async fn callback_paths_with_variable_segments_use_the_route_pattern() {
    let pattern = "/payments/{prefix:.*}/redirect/token/{callback_token}/response/{connector}";
    let app = test::init_service(
        App::new().service(web::resource(pattern).route(web::get().to(return_telemetry_path))),
    )
    .await;
    let request = test::TestRequest::get()
            .uri("/payments/pay_123/merchant_456/redirect/token/d9dc53cc-d1a1-4f57-9077-d20c2d614daa/response/stripe")
            .to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success());
    let body = test::read_body(response).await;
    assert_eq!(std::str::from_utf8(&body).expect("telemetry path"), pattern);
}
