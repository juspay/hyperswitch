mod utils;

#[actix_web::test]
#[ignore]
async fn card_issuer_success() {
    Box::pin(utils::setup()).await;

    let admin_api_key = ("api-key", "test_admin");
    let issuer_name = "STATE BANK OF INDIA";
    let updated_name = "STATE BANK OF INDIA UPDATED";

    let create_request = serde_json::json!({
        "issuer_name": issuer_name,
    });

    let update_request = serde_json::json!({
        "issuer_name": updated_name,
    });

    let client = awc::Client::default();
    let mut response;
    let mut response_body;

    // create card issuer
    response = client
        .post("http://127.0.0.1:8080/card_issuers")
        .insert_header(admin_api_key)
        .send_json(&create_request)
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-create: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    let issuer_id = response_body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    // update card issuer
    response = client
        .put(format!("http://127.0.0.1:8080/card_issuers/{issuer_id}"))
        .insert_header(admin_api_key)
        .send_json(&update_request)
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-update: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
    assert_eq!(
        response_body.get("issuer_name").and_then(|v| v.as_str()),
        Some(updated_name)
    );

    // list card issuers
    response = client
        .get("http://127.0.0.1:8080/card_issuers")
        .insert_header(admin_api_key)
        .send()
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-list: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    // list with query filter
    response = client
        .get("http://127.0.0.1:8080/card_issuers?query=STATE")
        .insert_header(admin_api_key)
        .send()
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-list-filtered: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
    assert!(!response_body
        .get("issuers")
        .and_then(|v| v.as_array())
        .unwrap()
        .is_empty());
}

#[actix_web::test]
#[ignore]
async fn card_issuer_failure() {
    Box::pin(utils::setup()).await;

    let admin_api_key = ("api-key", "test_admin");
    let issuer_name = "HDFC BANK";

    let create_request = serde_json::json!({
        "issuer_name": issuer_name,
    });

    let client = awc::Client::default();
    let mut response;
    let mut response_body;

    // create card issuer
    response = client
        .post("http://127.0.0.1:8080/card_issuers")
        .insert_header(admin_api_key)
        .send_json(&create_request)
        .await
        .unwrap();
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    // duplicate insert should fail
    response = client
        .post("http://127.0.0.1:8080/card_issuers")
        .insert_header(admin_api_key)
        .send_json(&create_request)
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-duplicate: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::BAD_REQUEST);

    // update with non-existent id should fail
    response = client
        .put("http://127.0.0.1:8080/card_issuers/non_existent_id")
        .insert_header(admin_api_key)
        .send_json(&serde_json::json!({ "issuer_name": "NEW NAME" }))
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-update-not-found: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::NOT_FOUND);

    // create with empty name should fail
    response = client
        .post("http://127.0.0.1:8080/card_issuers")
        .insert_header(admin_api_key)
        .send_json(&serde_json::json!({ "issuer_name": "" }))
        .await
        .unwrap();
    response_body = response.json::<serde_json::Value>().await.unwrap();
    println!("card-issuer-empty-name: {response:?} : {response_body:?}");
    assert_eq!(
        response.status(),
        awc::http::StatusCode::UNPROCESSABLE_ENTITY
    );
}
