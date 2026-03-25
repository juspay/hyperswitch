use actix_web::test::TestRequest;
use api_models::payouts::PayoutListConstraints;
use serde_json::Value;

mod utils;

async fn call_payout_list(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<impl actix_http::body::MessageBody>,
        Error = actix_web::Error,
    >,
    request: actix_http::Request,
) -> Value {
    let response = actix_web::test::call_service(app, request).await;
    let body = actix_web::test::read_body(response).await;
    serde_json::from_slice(&body).expect("Failed to parse JSON response")
}

/// Test that /payouts/profile/list returns total_count: 0 for an empty profile
#[actix_web::test]
async fn test_payouts_profile_list_empty_returns_zero_total_count() {
    let app = Box::pin(utils::mk_service()).await;

    // Create a request to the profile list endpoint with no payouts
    let constraints = PayoutListConstraints {
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: 10,
        created: None,
        time_range: None,
    };

    let request = TestRequest::get()
        .uri("/payouts/profile/list")
        .set_json(&constraints)
        .to_request();

    let response = call_payout_list(&app, request).await;

    // Verify the response structure has total_count field and it's 0 for empty profile
    assert_eq!(response["size"], 0);
    assert!(response["data"].as_array().map(|a| a.is_empty()).unwrap_or(false));
    assert_eq!(response["total_count"], 0);
}

#[actix_web::test]
async fn test_payouts_profile_list_returns_correct_total_count() {
    let app = Box::pin(utils::mk_service()).await;

    let constraints = PayoutListConstraints {
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: 10,
        created: None,
        time_range: None,
    };

    let request = TestRequest::get()
        .uri("/payouts/profile/list")
        .set_json(&constraints)
        .to_request();

    let response = call_payout_list(&app, request).await;

    assert!(response.get("total_count").is_some());
    let size = response["size"].as_u64().unwrap_or(0) as i64;
    let total_count = response["total_count"].as_i64().unwrap_or(0);
    assert!(total_count >= size);
}

#[actix_web::test]
async fn test_payouts_merchant_list_returns_actual_total_count() {
    let app = Box::pin(utils::mk_service()).await;

    let constraints = PayoutListConstraints {
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: 10,
        created: None,
        time_range: None,
    };

    let request = TestRequest::get()
        .uri("/payouts/list")
        .set_json(&constraints)
        .to_request();

    let response = call_payout_list(&app, request).await;

    assert!(response.get("total_count").is_some());
    let size = response["size"].as_u64().unwrap_or(0) as i64;
    let total_count = response["total_count"].as_i64().unwrap_or(0);
    assert!(total_count >= size);
}

#[actix_web::test]
async fn test_payouts_profile_list_response_structure() {
    let app = Box::pin(utils::mk_service()).await;

    let constraints = PayoutListConstraints {
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: 10,
        created: None,
        time_range: None,
    };

    let request = TestRequest::get()
        .uri("/payouts/profile/list")
        .set_json(&constraints)
        .to_request();

    let response = call_payout_list(&app, request).await;

    let size = response["size"].as_u64().unwrap_or(0) as usize;
    let data_len = response["data"].as_array().map(|a| a.len()).unwrap_or(0);
    assert_eq!(size, data_len);
    assert!(response.get("total_count").is_some());
}

#[actix_web::test]
async fn test_payouts_total_count_consistency() {
    let app = Box::pin(utils::mk_service()).await;

    let constraints = PayoutListConstraints {
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: 10,
        created: None,
        time_range: None,
    };

    let profile_request = TestRequest::get()
        .uri("/payouts/profile/list")
        .set_json(&constraints)
        .to_request();
    let profile_response = call_payout_list(&app, profile_request).await;

    let merchant_request = TestRequest::get()
        .uri("/payouts/list")
        .set_json(&constraints)
        .to_request();
    let merchant_response = call_payout_list(&app, merchant_request).await;

    assert!(profile_response.get("total_count").is_some());
    assert!(merchant_response.get("total_count").is_some());

    let profile_count = profile_response["total_count"].as_i64().unwrap_or(0);
    let merchant_count = merchant_response["total_count"].as_i64().unwrap_or(0);
    assert!(profile_count <= merchant_count);
}

#[actix_web::test]
#[ignore]
async fn payouts_todo() {
    Box::pin(utils::setup()).await;

    let client = awc::Client::default();
    let mut response;
    let mut response_body;
    let get_endpoints = vec!["retrieve", "accounts"];
    let post_endpoints = vec!["create", "update", "reverse", "cancel"];

    for endpoint in get_endpoints {
        response = client
            .get(format!("http://127.0.0.1:8080/payouts/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }

    for endpoint in post_endpoints {
        response = client
            .post(format!("http://127.0.0.1:8080/payouts/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }
}
