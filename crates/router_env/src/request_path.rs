//! Request paths for telemetry. Keep resource IDs while hiding callback tokens.

use std::borrow::Cow;

use actix_web::HttpRequest;

/// Use the concrete path except for a matched callback token. An unmatched path
/// has no trustworthy token position, so its path is omitted from telemetry.
pub fn path_for_telemetry(request: &HttpRequest) -> Cow<'_, str> {
    match request.match_pattern() {
        Some(pattern) if pattern.ends_with("/callback/{callback_token}") => {
            let Some((prefix, _token)) = request.path().rsplit_once('/') else {
                return Cow::Borrowed("/unmatched");
            };
            Cow::Owned(format!("{prefix}/{{callback_token}}"))
        }
        Some(pattern) if pattern.contains("{callback_token}") => Cow::Owned(pattern),
        Some(_) => Cow::Borrowed(request.path()),
        None => Cow::Borrowed("/unmatched"),
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{test, web, App, HttpResponse};

    use super::*;

    async fn return_telemetry_path(request: HttpRequest) -> HttpResponse {
        HttpResponse::Ok().body(path_for_telemetry(&request).into_owned())
    }

    #[actix_web::test]
    async fn callback_routes_hide_only_the_token() {
        const TOKEN: &str = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa";
        let app = test::init_service(App::new().configure(|config| {
            for pattern in [
                "/payments/redirect/{payment_id}/{merchant_id}/{attempt_id}/callback/{callback_token}",
                "/payments/{payment_id}/{merchant_id}/redirect/response/{connector}/callback/{callback_token}",
                "/payments/{payment_id}/{merchant_id}/redirect/response/{connector}/{creds_identifier}/callback/{callback_token}",
                "/payments/{payment_id}/{merchant_id}/redirect/complete/{connector}/callback/{callback_token}",
                "/payments/{payment_id}/{merchant_id}/redirect/complete/{connector}/{creds_identifier}/callback/{callback_token}",
                "/payments/{payment_id}/{merchant_id}/authorize/{connector}/callback/{callback_token}",
            ] {
                config.service(web::resource(pattern).route(web::get().to(return_telemetry_path)));
            }
        }))
        .await;

        for prefix in [
            "/payments/redirect/pay_123/merchant_456/attempt_789",
            "/payments/pay_123/merchant_456/redirect/response/stripe",
            "/payments/pay_123/merchant_456/redirect/response/stripe/creds_012",
            "/payments/pay_123/merchant_456/redirect/complete/stripe",
            "/payments/pay_123/merchant_456/redirect/complete/stripe/creds_012",
            "/payments/pay_123/merchant_456/authorize/stripe",
        ] {
            let request = test::TestRequest::get()
                .uri(&format!("{prefix}/callback/{TOKEN}"))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert!(response.status().is_success(), "{prefix}");
            let body = test::read_body(response).await;
            assert_eq!(body, format!("{prefix}/callback/{{callback_token}}"));
        }
    }
}
