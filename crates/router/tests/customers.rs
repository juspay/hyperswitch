#![allow(clippy::unwrap_used)]

mod utils;

// setting the connector in environment variables doesn't work when run in parallel. Neither does passing the paymentid
// do we'll test refund and payment in same tests and later implement thread_local variables.
// When test-connector feature is enabled, you can pass the connector name in description

#[actix_web::test]
#[ignore]
// verify the API-KEY/merchant id has stripe as first choice
async fn customer_success() {
    Box::pin(utils::setup()).await;

    let customer_id = format!("customer_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "MySecretApiKey");
    let name = "Doe";
    let new_name = "new Doe";

    let request = serde_json::json!({
        "customer_id" : customer_id,
        "name" : name,
    });

    let update_request = serde_json::json!({
        "name" : new_name,
    });

    let client = awc::Client::default();
    let mut response;
    let mut response_body;

    // create customer
    response = client
        .post("http://127.0.0.1:8080/customers")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("customer-create: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    // retrieve customer
    response = client
        .get(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();
    response_body = response.body().await;
    println!("customer-retrieve: {response:?} =:= {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    // update customer
    response = client
        .post(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send_json(&update_request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("customer-update: {response:?} =:= {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    // delete customer
    response = client
        .delete(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();
    response_body = response.body().await;
    println!("customer-delete : {response:?} =:= {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
}

#[actix_web::test]
#[ignore]
// verify the API-KEY/merchant id has stripe as first choice
async fn customer_failure() {
    Box::pin(utils::setup()).await;

    let customer_id = format!("customer_{}", uuid::Uuid::new_v4());
    let api_key = ("api-key", "MySecretApiKey");

    let mut request = serde_json::json!({
        "email" : "abcd",
    });

    let client = awc::Client::default();
    let mut response;
    let mut response_body;

    // insert the customer with invalid email when id not found
    response = client
        .post("http://127.0.0.1:8080/customers")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(
        response.status(),
        awc::http::StatusCode::UNPROCESSABLE_ENTITY
    );

    // retrieve a customer with customer id which is not in DB
    response = client
        .post(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::BAD_REQUEST);

    // update customer id with customer id which is not in DB
    response = client
        .post(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(
        response.status(),
        awc::http::StatusCode::UNPROCESSABLE_ENTITY
    );

    // delete a customer with customer id which is not in DB
    response = client
        .delete(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::BAD_REQUEST);

    // email validation for customer update
    request = serde_json::json!({ "customer_id": customer_id });

    response = client
        .post("http://127.0.0.1:8080/customers")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);

    request = serde_json::json!({
        "email": "abch"
    });
    response = client
        .post(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(
        response.status(),
        awc::http::StatusCode::UNPROCESSABLE_ENTITY
    );

    // address validation
    request = serde_json::json!({
        "email": "abch"
    });
    response = client
        .post(format!("http://127.0.0.1:8080/customers/{customer_id}"))
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    response_body = response.body().await;
    println!("{response:?} : {response_body:?}");
    assert_eq!(
        response.status(),
        awc::http::StatusCode::UNPROCESSABLE_ENTITY
    );
}
