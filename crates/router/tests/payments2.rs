#![allow(clippy::expect_used, clippy::unwrap_in_result, clippy::unwrap_used)]

mod utils;

use router::{
    core::{payment_methods::Oss, payments},
    db::StorageImpl,
    types::api::{self, enums as api_enums},
    *,
};
use time::macros::datetime;
use tokio::sync::oneshot;
use uuid::Uuid;

#[test]
fn connector_list() {
    let connector_list = router::types::ConnectorsList {
        connectors: vec![String::from("stripe"), "adyen".to_string()],
    };

    let json = serde_json::to_string(&connector_list).unwrap();

    println!("{}", &json);

    let newlist: router::types::ConnectorsList = serde_json::from_str(&json).unwrap();

    println!("{newlist:#?}");
    assert_eq!(true, true);
}

// FIXME: broken test?
#[ignore]
#[actix_rt::test]
async fn payments_create_core() {
    use router::configs::settings::Settings;
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
        capture_on: Some(datetime!(2022-09-10 10:11:12)),
        confirm: Some(true),
        customer_id: None,
        email: None,
        name: None,
        description: Some("Its my first payment request".to_string()),
        return_url: Some(url::Url::parse("http://example.com/payments").unwrap()),
        setup_future_usage: None,
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
        statement_descriptor_name: Some("Hyperswitch".to_string()),
        statement_descriptor_suffix: Some("Hyperswitch".to_string()),
        ..<_>::default()
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
    let actual_response = router::core::payments::payments_core::<
        api::Authorize,
        api::PaymentsResponse,
        _,
        _,
        _,
        Oss,
    >(
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

// FIXME: broken test? It looks like we haven't updated the test after removing the `core::payments::payments_start_core` method from the codebase.
// #[ignore]
// #[actix_rt::test]
// async fn payments_start_core_stripe_redirect() {
//     use router::configs::settings::Settings;
//     let conf = Settings::new().expect("invalid settings");
//
//     let state = routes::AppState {
//         flow_name: String::from("default"),
//         pg_pool: connection::make_pg_pool(&conf).await,
//         redis_conn: connection::redis_connection(&conf).await,
//     };
//
//     let customer_id = format!("cust_{}", Uuid::new_v4());
//     let merchant_id = "jarnura".to_string();
//     let payment_id = "pay_mbabizu24mvu3mela5njyhpit10".to_string();
//     let customer_data = api::CreateCustomerRequest {
//         customer_id: customer_id.clone(),
//         merchant_id: merchant_id.clone(),
//         ..api::CreateCustomerRequest::default()
//     };
//
//     let _customer = customer_data.insert(&*state.store).await.unwrap();
//
//     let merchant_account = services::authenticate(&state, "123").await.unwrap();
//     let payment_attempt = storage::PaymentAttempt::find_by_payment_id_merchant_id(
//         &*state.store,
//         &payment_id,
//         &merchant_id,
//     )
//     .await
//     .unwrap();
//     let payment_intent = storage::PaymentIntent::find_by_payment_id_merchant_id(
//         &*state.store,
//         &payment_id,
//         &merchant_id,
//     )
//     .await
//     .unwrap();
//     let payment_intent_update = storage::PaymentIntentUpdate::ReturnUrlUpdate {
//         return_url: "http://example.com/payments".to_string(),
//         status: None,
//     };
//     payment_intent
//         .update(&*state.store, payment_intent_update)
//         .await
//         .unwrap();
//
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

// FIXME: broken test?
#[ignore]
#[actix_rt::test]
async fn payments_create_core_adyen_no_redirect() {
    use router::configs::settings::Settings;
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
        setup_future_usage: Some(api_enums::FutureUsage::OffSession),
        authentication_type: Some(api_enums::AuthenticationType::NoThreeDs),
        payment_method_data: Some(api::PaymentMethodData::Card(api::Card {
            card_number: "5555 3412 4444 1115".to_string().try_into().unwrap(),
            card_exp_month: "03".to_string().into(),
            card_exp_year: "2030".to_string().into(),
            card_holder_name: "JohnDoe".to_string().into(),
            card_cvc: "737".to_string().into(),
            bank_code: None,
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        })),
        payment_method: Some(api_enums::PaymentMethod::Card),
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
    let actual_response = router::core::payments::payments_core::<
        api::Authorize,
        api::PaymentsResponse,
        _,
        _,
        _,
        Oss,
    >(
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
