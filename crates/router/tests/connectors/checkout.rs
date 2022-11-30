use std::marker::PhantomData;

use router::{
    core::payments,
    routes::AppState,
    types::{self, api, storage::enums, PaymentAddress},
};

use crate::connector_auth::ConnectorAuthentication;

fn construct_payment_router_data() -> types::PaymentsRouterData {
    let auth = ConnectorAuthentication::new()
        .checkout
        .expect("Missing Checkout connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: "checkout".to_string(),
        connector: "checkout".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        amount: 100,
        orca_return_url: None,
        currency: enums::Currency::USD,
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethodType::Card,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::PaymentsRequestData {
            payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                card_number: "4242424242424242".to_string().into(),
                card_exp_month: "10".to_string().into(),
                card_exp_year: "35".to_string().into(),
                card_holder_name: "John Doe".to_string().into(),
                card_cvc: "123".to_string().into(),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
        },
        response: None,
        payment_method_id: None,
        error_response: None,
        address: PaymentAddress::default(),
    }
}

fn construct_refund_router_data<F>() -> types::RefundsRouterData<F> {
    let auth = ConnectorAuthentication::new()
        .checkout
        .expect("Missing Checkout connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: "checkout".to_string(),
        connector: "checkout".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        amount: 100,
        currency: enums::Currency::USD,
        orca_return_url: None,
        payment_method: enums::PaymentMethodType::Card,
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::RefundsRequestData {
            refund_id: uuid::Uuid::new_v4().to_string(),
            payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                card_number: "4242424242424242".to_string().into(),
                card_exp_month: "10".to_string().into(),
                card_exp_year: "35".to_string().into(),
                card_holder_name: "John Doe".to_string().into(),
                card_cvc: "123".to_string().into(),
            }),
            connector_transaction_id: String::new(),
            refund_amount: 10,
        },
        response: None,
        payment_method_id: None,
        error_response: None,
        address: PaymentAddress::default(),
    }
}

#[actix_web::test]
async fn test_checkout_payment_success() {
    use router::{configs::settings::Settings, connector::Checkout, services};

    let conf = Settings::new().unwrap();
    static CV: Checkout = Checkout;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Checkout,
    };
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();

    println!("{response:?}");

    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
}

#[actix_web::test]
async fn test_checkout_refund_success() {
    // Successful payment
    use router::{configs::settings::Settings, connector::Checkout, services};

    let conf = Settings::new().expect("invalid settings");
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };
    static CV: Checkout = Checkout;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Checkout,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();

    println!("{response:?}");

    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
    // Successful refund
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut refund_request = construct_refund_router_data();
    refund_request.request.connector_transaction_id =
        response.response.unwrap().connector_transaction_id;

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &refund_request,
        payments::CallConnectorAction::Trigger,
    )
    .await;

    let response = response.unwrap();
    println!("{response:?}");

    assert!(
        response.response.unwrap().refund_status == enums::RefundStatus::Success,
        "The refund failed"
    );
}

#[actix_web::test]
async fn test_checkout_payment_failure() {
    use router::{configs::settings::Settings, connector::Checkout, services};

    let conf = Settings::new().expect("invalid settings");
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };
    static CV: Checkout = Checkout;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Checkout,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let mut request = construct_payment_router_data();
    request.connector_auth_type = types::ConnectorAuthType::BodyKey {
        api_key: "".to_string(),
        key1: "".to_string(),
    };
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await;
    assert!(response.is_err(), "The payment passed");
}
#[actix_web::test]
async fn test_checkout_refund_failure() {
    use router::{configs::settings::Settings, connector::Checkout, services};

    let conf = Settings::new().expect("invalid settings");
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };
    static CV: Checkout = Checkout;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Checkout,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();

    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
    // Unsuccessful refund
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut refund_request = construct_refund_router_data();
    refund_request.request.connector_transaction_id =
        response.response.unwrap().connector_transaction_id;

    // Higher amout than that of payment
    refund_request.request.refund_amount = 696969;
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &refund_request,
        payments::CallConnectorAction::Trigger,
    )
    .await;

    println!("{response:?}");
    let response = response.unwrap();
    assert!(response.error_response.is_some());

    let code = response.error_response.unwrap().code;
    assert_eq!(code, "refund_amount_exceeds_balance");
}
