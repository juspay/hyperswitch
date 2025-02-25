#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::print_stdout,
    unused_imports
)]

mod utils;

use std::{borrow::Cow, sync::Arc};

use common_utils::{id_type, types::MinorUnit};
use router::{
    core::payments,
    db::StorageImpl,
    types::api::{self, enums as api_enums},
    *,
};
use time::macros::datetime;
use tokio::sync::oneshot;
use uuid::Uuid;

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

#[cfg(feature = "v1")]
// FIXME: broken test?
#[ignore]
#[actix_rt::test]
async fn payments_create_core() {
    use router::configs::settings::Settings;
    let conf = Settings::new().expect("invalid settings");
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let app_state = Box::pin(routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;

    let merchant_id = id_type::MerchantId::try_from(Cow::from("juspay_merchant")).unwrap();

    let state = Arc::new(app_state)
        .get_session_state(
            &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .unwrap();

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
        .await
        .unwrap();

    let payment_id =
        id_type::PaymentId::try_from(Cow::Borrowed("pay_mbabizu24mvu3mela5njyhpit10")).unwrap();

    let req = api::PaymentsRequest {
        payment_id: Some(api::PaymentIdType::PaymentIntentId(payment_id.clone())),
        merchant_id: Some(merchant_id.clone()),
        amount: Some(MinorUnit::new(6540).into()),
        currency: Some(api_enums::Currency::USD),
        capture_method: Some(api_enums::CaptureMethod::Automatic),
        amount_to_capture: Some(MinorUnit::new(6540)),
        capture_on: Some(datetime!(2022-09-10 10:11:12)),
        confirm: Some(true),
        customer_id: None,
        email: None,
        name: None,
        description: Some("Its my first payment request".to_string()),
        return_url: Some(url::Url::parse("http://example.com/payments").unwrap()),
        setup_future_usage: None,
        authentication_type: Some(api_enums::AuthenticationType::NoThreeDs),
        payment_method_data: Some(api::PaymentMethodDataRequest {
            payment_method_data: Some(api::PaymentMethodData::Card(api::Card {
                card_number: "4242424242424242".to_string().try_into().unwrap(),
                card_exp_month: "10".to_string().into(),
                card_exp_year: "35".to_string().into(),
                card_holder_name: Some(masking::Secret::new("Arun Raj".to_string())),
                card_cvc: "123".to_string().into(),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(masking::Secret::new("nick_name".into())),
            })),
            billing: None,
        }),
        payment_method: Some(api_enums::PaymentMethod::Card),
        shipping: Some(api::Address {
            address: None,
            phone: None,
            email: None,
        }),
        billing: Some(api::Address {
            address: None,
            phone: None,
            email: None,
        }),
        statement_descriptor_name: Some("Hyperswitch".to_string()),
        statement_descriptor_suffix: Some("Hyperswitch".to_string()),
        ..<_>::default()
    };

    let expected_response = api::PaymentsResponse {
        payment_id,
        status: api_enums::IntentStatus::Succeeded,
        amount: MinorUnit::new(6540),
        amount_capturable: MinorUnit::new(0),
        amount_received: None,
        client_secret: None,
        created: None,
        currency: "USD".to_string(),
        customer_id: None,
        description: Some("Its my first payment request".to_string()),
        refunds: None,
        mandate_id: None,
        merchant_id,
        net_amount: MinorUnit::new(6540),
        connector: None,
        customer: None,
        disputes: None,
        attempts: None,
        captures: None,
        mandate_data: None,
        setup_future_usage: None,
        off_session: None,
        capture_on: None,
        capture_method: None,
        payment_method: None,
        payment_method_data: None,
        payment_token: None,
        shipping: None,
        billing: None,
        order_details: None,
        email: None,
        name: None,
        phone: None,
        return_url: None,
        authentication_type: None,
        statement_descriptor_name: None,
        statement_descriptor_suffix: None,
        next_action: None,
        cancellation_reason: None,
        error_code: None,
        error_message: None,
        unified_code: None,
        unified_message: None,
        payment_experience: None,
        payment_method_type: None,
        connector_label: None,
        business_country: None,
        business_label: None,
        business_sub_label: None,
        allowed_payment_method_types: None,
        ephemeral_key: None,
        manual_retry_allowed: None,
        connector_transaction_id: None,
        frm_message: None,
        metadata: None,
        connector_metadata: None,
        feature_metadata: None,
        reference_id: None,
        payment_link: None,
        profile_id: None,
        surcharge_details: None,
        attempt_count: 1,
        merchant_decision: None,
        merchant_connector_id: None,
        incremental_authorization_allowed: None,
        authorization_count: None,
        incremental_authorizations: None,
        external_authentication_details: None,
        external_3ds_authentication_attempted: None,
        expires_on: None,
        fingerprint: None,
        browser_info: None,
        payment_method_id: None,
        payment_method_status: None,
        updated: None,
        split_payments: None,
        frm_metadata: None,
        merchant_order_reference_id: None,
        capture_before: None,
        extended_authorization_applied: None,
        order_tax_amount: None,
        connector_mandate_id: None,
        shipping_cost: None,
        card_discovery: None,
    };

    let expected_response =
        services::ApplicationResponse::JsonWithHeaders((expected_response, vec![]));
    let actual_response = Box::pin(payments::payments_core::<
        api::Authorize,
        api::PaymentsResponse,
        _,
        _,
        _,
        payments::PaymentData<api::Authorize>,
    >(
        state.clone(),
        state.get_req_state(),
        merchant_account,
        None,
        key_store,
        payments::PaymentCreate,
        req,
        services::AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
        None,
    ))
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

#[cfg(feature = "v1")]
// FIXME: broken test?
#[ignore]
#[actix_rt::test]
async fn payments_create_core_adyen_no_redirect() {
    use router::configs::settings::Settings;
    let conf = Settings::new().expect("invalid settings");
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let app_state = Box::pin(routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();

    let customer_id = format!("cust_{}", Uuid::new_v4());
    let merchant_id = id_type::MerchantId::try_from(Cow::from("juspay_merchant")).unwrap();
    let payment_id =
        id_type::PaymentId::try_from(Cow::Borrowed("pay_mbabizu24mvu3mela5njyhpit10")).unwrap();

    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .unwrap();

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
        .await
        .unwrap();

    let req = api::PaymentsRequest {
        payment_id: Some(api::PaymentIdType::PaymentIntentId(payment_id.clone())),
        merchant_id: Some(merchant_id.clone()),
        amount: Some(MinorUnit::new(6540).into()),
        currency: Some(api_enums::Currency::USD),
        capture_method: Some(api_enums::CaptureMethod::Automatic),
        amount_to_capture: Some(MinorUnit::new(6540)),
        capture_on: Some(datetime!(2022-09-10 10:11:12)),
        confirm: Some(true),
        customer_id: Some(id_type::CustomerId::try_from(Cow::from(customer_id)).unwrap()),
        description: Some("Its my first payment request".to_string()),
        return_url: Some(url::Url::parse("http://example.com/payments").unwrap()),
        setup_future_usage: Some(api_enums::FutureUsage::OffSession),
        authentication_type: Some(api_enums::AuthenticationType::NoThreeDs),
        payment_method_data: Some(api::PaymentMethodDataRequest {
            payment_method_data: Some(api::PaymentMethodData::Card(api::Card {
                card_number: "5555 3412 4444 1115".to_string().try_into().unwrap(),
                card_exp_month: "03".to_string().into(),
                card_exp_year: "2030".to_string().into(),
                card_holder_name: Some(masking::Secret::new("JohnDoe".to_string())),
                card_cvc: "737".to_string().into(),
                bank_code: None,
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                nick_name: Some(masking::Secret::new("nick_name".into())),
            })),
            billing: None,
        }),

        payment_method: Some(api_enums::PaymentMethod::Card),
        shipping: Some(api::Address {
            address: None,
            phone: None,
            email: None,
        }),
        billing: Some(api::Address {
            address: None,
            phone: None,
            email: None,
        }),
        statement_descriptor_name: Some("Juspay".to_string()),
        statement_descriptor_suffix: Some("Router".to_string()),
        ..Default::default()
    };

    let expected_response = services::ApplicationResponse::JsonWithHeaders((
        api::PaymentsResponse {
            payment_id: payment_id.clone(),
            status: api_enums::IntentStatus::Processing,
            amount: MinorUnit::new(6540),
            amount_capturable: MinorUnit::new(0),
            amount_received: None,
            client_secret: None,
            created: None,
            currency: "USD".to_string(),
            customer_id: None,
            description: Some("Its my first payment request".to_string()),
            refunds: None,
            mandate_id: None,
            merchant_id,
            net_amount: MinorUnit::new(6540),
            connector: None,
            customer: None,
            disputes: None,
            attempts: None,
            captures: None,
            mandate_data: None,
            setup_future_usage: None,
            off_session: None,
            capture_on: None,
            capture_method: None,
            payment_method: None,
            payment_method_data: None,
            payment_token: None,
            shipping: None,
            billing: None,
            order_details: None,
            email: None,
            name: None,
            phone: None,
            return_url: None,
            authentication_type: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            next_action: None,
            cancellation_reason: None,
            error_code: None,
            error_message: None,
            unified_code: None,
            unified_message: None,
            payment_experience: None,
            payment_method_type: None,
            connector_label: None,
            business_country: None,
            business_label: None,
            business_sub_label: None,
            allowed_payment_method_types: None,
            ephemeral_key: None,
            manual_retry_allowed: None,
            connector_transaction_id: None,
            frm_message: None,
            metadata: None,
            connector_metadata: None,
            feature_metadata: None,
            reference_id: None,
            payment_link: None,
            profile_id: None,
            surcharge_details: None,
            attempt_count: 1,
            merchant_decision: None,
            merchant_connector_id: None,
            incremental_authorization_allowed: None,
            authorization_count: None,
            incremental_authorizations: None,
            external_authentication_details: None,
            external_3ds_authentication_attempted: None,
            expires_on: None,
            fingerprint: None,
            browser_info: None,
            payment_method_id: None,
            payment_method_status: None,
            updated: None,
            split_payments: None,
            frm_metadata: None,
            merchant_order_reference_id: None,
            capture_before: None,
            extended_authorization_applied: None,
            order_tax_amount: None,
            connector_mandate_id: None,
            shipping_cost: None,
            card_discovery: None,
        },
        vec![],
    ));
    let actual_response = Box::pin(payments::payments_core::<
        api::Authorize,
        api::PaymentsResponse,
        _,
        _,
        _,
        payments::PaymentData<api::Authorize>,
    >(
        state.clone(),
        state.get_req_state(),
        merchant_account,
        None,
        key_store,
        payments::PaymentCreate,
        req,
        services::AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
        None,
    ))
    .await
    .unwrap();
    assert_eq!(expected_response, actual_response);
}
