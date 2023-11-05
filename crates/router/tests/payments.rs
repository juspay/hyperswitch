#![allow(clippy::expect_used, clippy::unwrap_in_result, clippy::unwrap_used)]

mod utils;

use router::{
    configs,
    core::{payment_methods::Oss, payments},
    db::StorageImpl,
    routes, services,
    types::{
        self,
        api::{self, enums as api_enums},
    },
};
use time::macros::datetime;
use tokio::sync::oneshot;
use uuid::Uuid;

// setting the connector in environment variables doesn't work when run in parallel. Neither does passing the paymentid
// do we'll test refund and payment in same tests and later implement thread_local variables.
// When test-connector feature is enabled, you can pass the connector name in description

#[actix_web::test]
#[ignore]
// verify the API-KEY/merchant id has stripe as first choice
async fn payments_create_stripe() {
    utils::setup().await;

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "MySecretApiKey");

    let request = serde_json::json!({
    "payment_id" : payment_id,
    "merchant_id" : "jarnura",
    "amount" : 1000,
    "currency" : "USD",
    "amount_to_capture" : 1000,
    "confirm" : true,
    "customer" : "test_customer",
    "customer_email" : "test@gmail.com",
    "customer_name" : "Test",
    "description" : "stripe",
    "return_url" : "https://juspay.in/",
    "payment_method_data" : {"card" : {"card_number":"4242424242424242","card_exp_month":"12","card_exp_year":"29","card_holder_name":"JohnDoe","card_cvc":"123"}},
    "payment_method" : "card",
    "statement_descriptor_name" : "Test Merchant",
    "statement_descriptor_suffix" : "US"
    });

    let refund_req = serde_json::json!({
            "amount" : 1000,
            "currency" : "USD",
            "refund_id" : "refund_123",
            "payment_id" : payment_id,
            "merchant_id" : "jarnura",
    });

    let client = awc::Client::default();

    let mut create_response = client
        .post("http://127.0.0.1:8080/payments/create")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    let create_response_body = create_response.body().await;
    println!("{create_response:?} : {create_response_body:?}");
    assert_eq!(create_response.status(), awc::http::StatusCode::OK);

    let mut retrieve_response = client
        .get("http://127.0.0.1:8080/payments/retrieve")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    let retrieve_response_body = retrieve_response.body().await;
    println!("{retrieve_response:?} =:= {retrieve_response_body:?}");
    assert_eq!(retrieve_response.status(), awc::http::StatusCode::OK);

    let mut refund_response = client
        .post("http://127.0.0.1:8080/refunds/create")
        .insert_header(api_key)
        .send_json(&refund_req)
        .await
        .unwrap();

    let refund_response_body = refund_response.body().await;
    println!("{refund_response:?} =:= {refund_response_body:?}");
    assert_eq!(refund_response.status(), awc::http::StatusCode::OK);
}

#[actix_web::test]
#[ignore]
// verify the API-KEY/merchant id has adyen as first choice
async fn payments_create_adyen() {
    utils::setup().await;

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "321");

    let request = serde_json::json!({
    "payment_id" : payment_id,
    "merchant_id" : "jarnura",
    "amount" : 1000,
    "currency" : "USD",
    "amount_to_capture" : 1000,
    "confirm" : true,
    "customer" : "test_customer",
    "customer_email" : "test@gmail.com",
    "customer_name" : "Test",
    "description" : "adyen",
    "return_url" : "https://juspay.in/",
    "payment_method_data" : {"card" : {"card_number":"5555 3412 4444 1115","card_exp_month":"03","card_exp_year":"2030","card_holder_name":"JohnDoe","card_cvc":"737"}},
    "payment_method" : "card",
    "statement_descriptor_name" : "Test Merchant",
    "statement_descriptor_suffix" : "US"
    });

    let refund_req = serde_json::json!({
            "amount" : 1000,
            "currency" : "USD",
            "refund_id" : "refund_123",
            "payment_id" : payment_id,
            "merchant_id" : "jarnura",
    });

    let client = awc::Client::default();

    let mut create_response = client
        .post("http://127.0.0.1:8080/payments/create")
        .insert_header(api_key) //API Key must have adyen as first choice
        .send_json(&request)
        .await
        .unwrap();
    let create_response_body = create_response.body().await;
    println!("{create_response:?} : {create_response_body:?}");
    assert_eq!(create_response.status(), awc::http::StatusCode::OK);

    let mut retrieve_response = client
        .get("http://127.0.0.1:8080/payments/retrieve")
        .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    let retrieve_response_body = retrieve_response.body().await;
    println!("{retrieve_response:?} =:= {retrieve_response_body:?}");
    assert_eq!(retrieve_response.status(), awc::http::StatusCode::OK);

    let mut refund_response = client
        .post("http://127.0.0.1:8080/refunds/create")
        .insert_header(api_key)
        .send_json(&refund_req)
        .await
        .unwrap();

    let refund_response_body = refund_response.body().await;
    println!("{refund_response:?} =:= {refund_response_body:?}");
    assert_eq!(refund_response.status(), awc::http::StatusCode::OK);
}

#[actix_web::test]
// verify the API-KEY/merchant id has stripe as first choice
#[ignore]
async fn payments_create_fail() {
    utils::setup().await;

    let payment_id = format!("test_{}", uuid::Uuid::new_v4());
    let api_key = ("API-KEY", "MySecretApiKey");

    let invalid_request = serde_json::json!({
    "description" : "stripe",
    });

    let request = serde_json::json!({
    "payment_id" : payment_id,
    "merchant_id" : "jarnura",
    "amount" : 1000,
    "currency" : "USD",
    "amount_to_capture" : 1000,
    "confirm" : true,
    "customer" : "test_customer",
    "customer_email" : "test@gmail.com",
    "customer_name" : "Test",
    "description" : "adyen",
    "return_url" : "https://juspay.in/",
    "payment_method_data" : {"card" : {"card_number":"5555 3412 4444 1115","card_exp_month":"03","card_exp_year":"2030","card_holder_name":"JohnDoe","card_cvc":"737"}},
    "payment_method" : "card",
    "statement_descriptor_name" : "Test Merchant",
    "statement_descriptor_suffix" : "US"
    });

    let client = awc::Client::default();

    let mut invalid_response = client
        .post("http://127.0.0.1:8080/payments/create")
        .insert_header(api_key)
        .send_json(&invalid_request)
        .await
        .unwrap();
    let invalid_response_body = invalid_response.body().await;
    println!("{invalid_response:?} : {invalid_response_body:?}");
    assert_eq!(
        invalid_response.status(),
        awc::http::StatusCode::BAD_REQUEST
    );

    let mut api_key_response = client
        .get("http://127.0.0.1:8080/payments/retrieve")
        // .insert_header(api_key)
        .send_json(&request)
        .await
        .unwrap();
    let api_key_response_body = api_key_response.body().await;
    println!("{api_key_response:?} =:= {api_key_response_body:?}");
    assert_eq!(
        api_key_response.status(),
        awc::http::StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
#[ignore]
async fn payments_todo() {
    utils::setup().await;

    let client = awc::Client::default();
    let mut response;
    let mut response_body;
    let _post_endpoints = ["123/update", "123/confirm", "cancel"];
    let get_endpoints = vec!["list"];

    for endpoint in get_endpoints {
        response = client
            .get(format!("http://127.0.0.1:8080/payments/{endpoint}"))
            .insert_header(("API-KEY", "MySecretApiKey"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }

    // for endpoint in post_endpoints {
    //     response = client
    //         .post(format!("http://127.0.0.1:8080/payments/{}", endpoint))
    //         .send()
    //         .await
    //         .unwrap();
    //     response_body = response.body().await;
    //     println!("{} =:= {:?} : {:?}", endpoint, response, response_body);
    //     assert_eq!(response.status(), awc::http::StatusCode::OK);
    // }
}

#[test]
fn connector_list() {
    let connector_list = types::ConnectorsList {
        connectors: vec![String::from("stripe"), "adyen".to_string()],
    };

    let json = serde_json::to_string(&connector_list).unwrap();

    println!("{}", &json);

    let newlist: types::ConnectorsList = serde_json::from_str(&json).unwrap();

    println!("{newlist:#?}");
    assert_eq!(true, true);
}

#[actix_rt::test]
#[ignore] // AWS
async fn payments_create_core() {
    use configs::settings::Settings;
    let conf = Settings::new().expect("invalid settings");
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            "juspay_merchant",
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .unwrap();

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id("juspay_merchant", &key_store)
        .await
        .unwrap();

    let req = api::PaymentsRequest {
        payment_id: Some(api::PaymentIdType::PaymentIntentId(
            "pay_mbabizu24mvu3mela5njyhpit10".to_string(),
        )),
        merchant_id: Some("jarnura".to_string()),
        amount: Some(6540.into()),
        currency: Some(api_enums::Currency::USD),
        capture_method: Some(api_enums::CaptureMethod::Automatic),
        amount_to_capture: Some(6540),
        capture_on: Some(datetime!(2022-09-10 11:12)),
        confirm: Some(true),
        customer_id: None,
        email: None,
        name: None,
        description: Some("Its my first payment request".to_string()),
        return_url: Some(url::Url::parse("http://example.com/payments").unwrap()),
        setup_future_usage: Some(api_enums::FutureUsage::OnSession),
        authentication_type: Some(api_enums::AuthenticationType::NoThreeDs),
        payment_method_data: Some(api::PaymentMethodData::Card(api::Card {
            card_number: "4242424242424242".to_string().try_into().unwrap(),
            card_exp_month: "10".to_string().into(),
            card_exp_year: "35".to_string().into(),
            card_holder_name: "Arun Raj".to_string().into(),
            card_cvc: "123".to_string().into(),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        })),
        payment_method: Some(api_enums::PaymentMethod::Card),
        shipping: Some(api::Address {
            address: None,
            phone: None,
        }),
        billing: Some(api::Address {
            address: None,
            phone: None,
        }),
        statement_descriptor_name: Some("Hyperswtich".to_string()),
        statement_descriptor_suffix: Some("Hyperswitch".to_string()),
        ..Default::default()
    };

    let expected_response = api::PaymentsResponse {
        payment_id: Some("pay_mbabizu24mvu3mela5njyhpit10".to_string()),
        status: api_enums::IntentStatus::Succeeded,
        amount: 6540,
        amount_capturable: None,
        amount_received: None,
        client_secret: None,
        created: None,
        currency: "USD".to_string(),
        customer_id: None,
        description: Some("Its my first payment request".to_string()),
        refunds: None,
        mandate_id: None,
        ..Default::default()
    };
    let expected_response =
        services::ApplicationResponse::JsonWithHeaders((expected_response, vec![]));
    let actual_response =
        payments::payments_core::<api::Authorize, api::PaymentsResponse, _, _, _, Oss>(
            state,
            merchant_account,
            key_store,
            payments::PaymentCreate,
            req,
            services::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            api::HeaderPayload::default(),
        )
        .await
        .unwrap();
    assert_eq!(expected_response, actual_response);
}

// #[actix_rt::test]
// async fn payments_start_core_stripe_redirect() {
//     use configs::settings::Settings;
//     let conf = Settings::new().expect("invalid settings");

//     let state = routes::AppState {
//         flow_name: String::from("default"),
//         pg_conn: connection::pg_connection_read(&conf),
//         redis_conn: connection::redis_connection(&conf).await,
//     };

//     let customer_id = format!("cust_{}", Uuid::new_v4());
//     let merchant_id = "jarnura".to_string();
//     let payment_id = "pay_mbabizu24mvu3mela5njyhpit10".to_string();
//     let customer_data = api::CreateCustomerRequest {
//         customer_id: customer_id.clone(),
//         merchant_id: merchant_id.clone(),
//         ..api::CreateCustomerRequest::default()
//     };

//     let _customer = customer_data.insert(&state.pg_conn).unwrap();

//     let merchant_account = services::authenticate(&state, "MySecretApiKey").unwrap();
//     let payment_attempt = storage::PaymentAttempt::find_by_payment_id_merchant_id(
//         &state.pg_conn,
//         &payment_id,
//         &merchant_id,
//     )
//     .unwrap();
//     let payment_intent = storage::PaymentIntent::find_by_payment_id_merchant_id(
//         &state.pg_conn,
//         &payment_id,
//         &merchant_id,
//     )
//     .unwrap();
//     let payment_intent_update = storage::PaymentIntentUpdate::ReturnUrlUpdate {
//         return_url: "http://example.com/payments".to_string(),
//         status: None,
//     };
//     payment_intent
//         .update(&state.pg_conn, payment_intent_update)
//         .unwrap();

//     let expected_response = services::ApplicationResponse::Form(services::RedirectForm {
//         url: "http://example.com/payments".to_string(),
//         method: services::Method::Post,
//         form_fields: HashMap::from([("payment_id".to_string(), payment_id.clone())]),
//     });
//     let actual_response = payments_start_core(
//         &state,
//         merchant_account,
//         api::PaymentsStartRequest {
//             payment_id,
//             merchant_id,
//             txn_id: payment_attempt.txn_id.to_owned(),
//         },
//     )
//     .await
//     .unwrap();
//     assert_eq!(expected_response, actual_response);
// }

#[actix_rt::test]
#[ignore]
async fn payments_create_core_adyen_no_redirect() {
    use crate::configs::settings::Settings;
    let conf = Settings::new().expect("invalid settings");
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;

    let customer_id = format!("cust_{}", Uuid::new_v4());
    let merchant_id = "arunraj".to_string();
    let payment_id = "pay_mbabizu24mvu3mela5njyhpit10".to_string();

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            "juspay_merchant",
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .unwrap();

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id("juspay_merchant", &key_store)
        .await
        .unwrap();

    let req = api::PaymentsRequest {
        payment_id: Some(api::PaymentIdType::PaymentIntentId(payment_id.clone())),
        merchant_id: Some(merchant_id.clone()),
        amount: Some(6540.into()),
        currency: Some(api_enums::Currency::USD),
        capture_method: Some(api_enums::CaptureMethod::Automatic),
        amount_to_capture: Some(6540),
        capture_on: Some(datetime!(2022-09-10 10:11:12)),
        confirm: Some(true),
        customer_id: Some(customer_id),
        description: Some("Its my first payment request".to_string()),
        return_url: Some(url::Url::parse("http://example.com/payments").unwrap()),
        setup_future_usage: Some(api_enums::FutureUsage::OnSession),
        authentication_type: Some(api_enums::AuthenticationType::NoThreeDs),
        payment_method_data: Some(api::PaymentMethodData::Card(api::Card {
            card_number: "5555 3412 4444 1115".to_string().try_into().unwrap(),
            card_exp_month: "03".to_string().into(),
            card_exp_year: "2030".to_string().into(),
            card_holder_name: "JohnDoe".to_string().into(),
            card_cvc: "737".to_string().into(),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        })),
        payment_method: Some(api_enums::PaymentMethod::Card),
        shipping: Some(api::Address {
            address: None,
            phone: None,
        }),
        billing: Some(api::Address {
            address: None,
            phone: None,
        }),
        statement_descriptor_name: Some("Juspay".to_string()),
        statement_descriptor_suffix: Some("Router".to_string()),
        ..Default::default()
    };

    let expected_response = services::ApplicationResponse::JsonWithHeaders((
        api::PaymentsResponse {
            payment_id: Some(payment_id.clone()),
            status: api_enums::IntentStatus::Processing,
            amount: 6540,
            amount_capturable: None,
            amount_received: None,
            client_secret: None,
            created: None,
            currency: "USD".to_string(),
            customer_id: None,
            description: Some("Its my first payment request".to_string()),
            refunds: None,
            mandate_id: None,
            ..Default::default()
        },
        vec![],
    ));
    let actual_response =
        payments::payments_core::<api::Authorize, api::PaymentsResponse, _, _, _, Oss>(
            state,
            merchant_account,
            key_store,
            payments::PaymentCreate,
            req,
            services::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            api::HeaderPayload::default(),
        )
        .await
        .unwrap();
    assert_eq!(expected_response, actual_response);
}
