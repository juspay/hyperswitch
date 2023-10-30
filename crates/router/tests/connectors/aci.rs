use std::{marker::PhantomData, str::FromStr};

use masking::Secret;
use router::{
    configs::settings::Settings,
    connector::aci,
    core::payments,
    db::StorageImpl,
    routes, services,
    types::{self, storage::enums, PaymentAddress},
};
use tokio::sync::oneshot;

use crate::{connector_auth::ConnectorAuthentication, utils};

fn construct_payment_router_data() -> types::PaymentsAuthorizeRouterData {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: String::from("aci"),
        customer_id: Some(String::from("aci")),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethod::Card,
        connector_auth_type: utils::to_connector_auth_type(auth.into()),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::PaymentsAuthorizeData {
            amount: 1000,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
                card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new("999".to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(masking::Secret::new("nick_name".into())),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: None,
            customer_id: None,
            surcharge_details: None,
        },
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
        address: PaymentAddress::default(),
        connector_meta_data: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,

        preprocessing_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        apple_pay_flow: None,
        external_latency: None,
    }
}

fn construct_refund_router_data<F>() -> types::RefundsRouterData<F> {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: String::from("aci"),
        customer_id: Some(String::from("aci")),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        payment_method: enums::PaymentMethod::Card,
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_auth_type: utils::to_connector_auth_type(auth.into()),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::RefundsData {
            payment_amount: 1000,
            currency: enums::Currency::USD,

            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 100,
            webhook_url: None,
            connector_metadata: None,
            reason: None,
            connector_refund_id: None,
            browser_info: None,
        },
        payment_method_id: None,
        response: Err(types::ErrorResponse::default()),
        address: PaymentAddress::default(),
        connector_meta_data: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,

        preprocessing_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        apple_pay_flow: None,
        external_latency: None,
    }
}

#[actix_web::test]

async fn payments_create_success() {
    let conf = Settings::new().unwrap();
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;

    static CV: aci::Aci = aci::Aci;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Aci,
        get_token: types::api::GetToken::Connector,
        merchant_connector_id: None,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
}

#[actix_web::test]
#[ignore]
async fn payments_create_failure() {
    {
        let conf = Settings::new().unwrap();
        static CV: aci::Aci = aci::Aci;
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        let connector = types::api::ConnectorData {
            connector: Box::new(&CV),
            connector_name: types::Connector::Aci,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        };
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let mut request = construct_payment_router_data();
        request.request.payment_method_data =
            types::api::PaymentMethodData::Card(types::api::Card {
                card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new("99".to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(masking::Secret::new("nick_name".into())),
            });

        let response = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .is_err();
        println!("{response:?}");
        assert!(response, "The payment was intended to fail but it passed");
    }
}

#[actix_web::test]

async fn refund_for_successful_payments() {
    let conf = Settings::new().unwrap();
    static CV: aci::Aci = aci::Aci;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Aci,
        get_token: types::api::GetToken::Connector,
        merchant_connector_id: None,
    };
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut refund_request = construct_refund_router_data();
    refund_request.request.connector_transaction_id = match response.response.unwrap() {
        types::PaymentsResponseData::TransactionResponse { resource_id, .. } => {
            resource_id.get_connector_transaction_id().unwrap()
        }
        _ => panic!("Connector transaction id not found"),
    };
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &refund_request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .unwrap();
    println!("{response:?}");
    assert!(
        response.response.unwrap().refund_status == enums::RefundStatus::Success,
        "The refund transaction failed"
    );
}

#[actix_web::test]
#[ignore]
async fn refunds_create_failure() {
    let conf = Settings::new().unwrap();
    static CV: aci::Aci = aci::Aci;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Aci,
        get_token: types::api::GetToken::Connector,
        merchant_connector_id: None,
    };
    let tx: oneshot::Sender<()> = oneshot::channel().0;
    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut request = construct_refund_router_data();
    request.request.connector_transaction_id = "1234".to_string();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .is_err();
    println!("{response:?}");
    assert!(response, "The refund was intended to fail but it passed");
}
