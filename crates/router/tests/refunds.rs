mod utils;

// setting the connector in environment variables doesn't work when run in parallel. Neither does passing the paymentid
// do we'll test refund and payment in same tests and later implement thread_local variables.
// When test-connector feature is enabled, you can pass the connector name in description

#[actix_web::test]
// verify the API-KEY/merchant id has stripe as first choice
async fn refund_create_fail_stripe() {
    utils::setup().await;

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "MySecretApiKey");

    let refund_req = serde_json::json!({
            "amount" : 10.00,
            "currency" : "USD",
            "refund_id" : "refund_123",
            "payment_id" : payment_id,
            "merchant_id" : "jarnura",
    });

    let client = awc::Client::default();

    let mut refund_response = client
        .post("http://127.0.0.1:8080/refunds/create")
        .insert_header(api_key)
        .send_json(&refund_req)
        .await
        .unwrap();

    let refund_response_body = refund_response.body().await;
    println!("{:?} =:= {:?}", refund_response, refund_response_body);
    assert_eq!(refund_response.status(), awc::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
// verify the API-KEY/merchant id has adyen as first choice
async fn refund_create_fail_adyen() {
    utils::setup().await;

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "321");

    let refund_req = serde_json::json!({
            "amount" : 10.00,
            "currency" : "USD",
            "refund_id" : "refund_123",
            "payment_id" : payment_id,
            "merchant_id" : "jarnura",
    });

    let client = awc::Client::default();

    let mut refund_response = client
        .post("http://127.0.0.1:8080/refunds/create")
        .insert_header(api_key)
        .send_json(&refund_req)
        .await
        .unwrap();

    let refund_response_body = refund_response.body().await;
    println!("{:?} =:= {:?}", refund_response, refund_response_body);
    assert_eq!(refund_response.status(), awc::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
#[ignore]
async fn refunds_todo() {
    utils::setup().await;

    let client = awc::Client::default();
    let mut response;
    let mut response_body;
    let get_endpoints = vec!["list"];
    let post_endpoints: Vec<&str> = vec![];

    for endpoint in get_endpoints {
        response = client
            .get(format!("http://127.0.0.1:8080/refunds/{}", endpoint))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{} =:= {:?} : {:?}", endpoint, response, response_body);
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }

    for endpoint in post_endpoints {
        response = client
            .post(format!("http://127.0.0.1:8080/refunds/{}", endpoint))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{} =:= {:?} : {:?}", endpoint, response, response_body);
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }
}
