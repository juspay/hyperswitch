#![allow(clippy::unwrap_used)]

use utils::{mk_service, AppClient};

mod utils;

// setting the connector in environment variables doesn't work when run in parallel. Neither does passing the paymentid
// do we'll test refund and payment in same tests and later implement thread_local variables.
// When test-connector feature is enabled, you can pass the connector name in description

#[actix_web::test]
// verify the API-KEY/merchant id has stripe as first choice
async fn refund_create_fail_stripe() {
    let app = Box::pin(mk_service()).await;
    let client = AppClient::guest();

    let user_client = client.user("321");

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let refund: serde_json::Value = user_client.create_refund(&app, &payment_id, 10).await;

    assert_eq!(refund.get("error").unwrap().get("message").unwrap(), "Access forbidden, invalid API key was used. Please create your new API key from the Dashboard Settings section.");
}

#[actix_web::test]
// verify the API-KEY/merchant id has adyen as first choice
async fn refund_create_fail_adyen() {
    let app = Box::pin(mk_service()).await;
    let client = AppClient::guest();

    let user_client = client.user("321");

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let refund: serde_json::Value = user_client.create_refund(&app, &payment_id, 10).await;

    assert_eq!(refund.get("error").unwrap().get("message").unwrap(), "Access forbidden, invalid API key was used. Please create your new API key from the Dashboard Settings section.");
}

#[actix_web::test]
#[ignore]
/// Asynchronously sends GET and POST requests to the refunds API endpoints
async fn refunds_todo() {
    Box::pin(utils::setup()).await;

    let client = awc::Client::default();
    let mut response;
    let mut response_body;
    let get_endpoints = vec!["list"];
    let post_endpoints: Vec<&str> = vec![];

    for endpoint in get_endpoints {
        response = client
            .get(format!("http://127.0.0.1:8080/refunds/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }

    for endpoint in post_endpoints {
        response = client
            .post(format!("http://127.0.0.1:8080/refunds/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }
}
