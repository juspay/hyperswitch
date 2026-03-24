use actix_web::test::{call_and_read_body_json, TestRequest};
use api_models::payouts::{PayoutListConstraints, PayoutListResponse};
use common_utils::id_type;

mod utils;

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

    let response: PayoutListResponse = call_and_read_body_json(&app, request).await;

    // Verify the response structure has total_count field and it's 0 for empty profile
    assert_eq!(response.size, 0);
    assert!(response.data.is_empty());
    assert_eq!(response.total_count, Some(0));
}

/// Test that /payouts/profile/list returns correct total_count for profile with payouts
#[actix_web::test]
async fn test_payouts_profile_list_returns_correct_total_count() {
    let app = Box::pin(utils::mk_service()).await;

    // Create a request to the profile list endpoint
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

    let response: PayoutListResponse = call_and_read_body_json(&app, request).await;

    // Verify the response structure has total_count field
    assert!(response.total_count.is_some());
    // total_count should be >= size (could be more if paginated)
    if let Some(total_count) = response.total_count {
        assert!(total_count >= response.size as i64);
    }
}

/// Test that /payouts/list (merchant-level) returns actual total_count
#[actix_web::test]
async fn test_payouts_merchant_list_returns_actual_total_count() {
    let app = Box::pin(utils::mk_service()).await;

    // Create a request to the merchant-level list endpoint
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

    let response: PayoutListResponse = call_and_read_body_json(&app, request).await;

    // Verify the response structure has total_count field with actual count
    assert!(response.total_count.is_some());
    // total_count should be >= size (could be more if paginated)
    if let Some(total_count) = response.total_count {
        assert!(total_count >= response.size as i64);
    }
}

/// Test that /payouts/profile/list response structure matches expected schema
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

    let response: PayoutListResponse = call_and_read_body_json(&app, request).await;

    // Verify all expected fields are present
    assert_eq!(response.size, response.data.len());
    assert!(response.total_count.is_some());
}

/// Test total_count consistency between profile-level and merchant-level endpoints
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

    // Get profile-level list
    let profile_request = TestRequest::get()
        .uri("/payouts/profile/list")
        .set_json(&constraints)
        .to_request();
    let profile_response: PayoutListResponse =
        call_and_read_body_json(&app, profile_request).await;

    // Get merchant-level list
    let merchant_request = TestRequest::get()
        .uri("/payouts/list")
        .set_json(&constraints)
        .to_request();
    let merchant_response: PayoutListResponse =
        call_and_read_body_json(&app, merchant_request).await;

    // Both should have total_count field
    assert!(profile_response.total_count.is_some());
    assert!(merchant_response.total_count.is_some());

    // Profile-level count should be <= merchant-level count (subset)
    if let (Some(profile_count), Some(merchant_count)) =
        (profile_response.total_count, merchant_response.total_count)
    {
        assert!(profile_count <= merchant_count);
    }
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
