use std::marker::PhantomData;

use masking::Secret;
use router::{
    configs::settings::Settings,
    connector::aci,
    core::payments,
    routes::AppState,
    services,
    types::{self, storage::enums, PaymentAddress},
};

use crate::connector_auth::ConnectorAuthentication;

fn construct_payment_router_data() -> types::PaymentsRouterData {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: String::from("aci"),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        amount: 1000,
        auth_type: enums::AuthenticationType::NoThreeDs,
        currency: enums::Currency::USD,
        payment_method: enums::PaymentMethodType::Card,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        orca_return_url: None,
        return_url: None,
        request: types::PaymentsRequestData {
            payment_method_data: types::api::PaymentMethod::Card(types::api::CCard {
                card_number: Secret::new("4200000000000000".to_string()),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new("999".to_string()),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
        },
        response: None,
        payment_method_id: None,
        error_response: None,
        address: PaymentAddress::default(),
    }
}

fn construct_refund_router_data<F>() -> types::RefundsRouterData<F> {
    let auth = ConnectorAuthentication::new()
        .aci
        .expect("Missing ACI connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: String::from("aci"),
        connector: "aci".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        amount: 1000,
        orca_return_url: None,
        currency: enums::Currency::USD,
        payment_method: enums::PaymentMethodType::Card,
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::RefundsRequestData {
            refund_id: uuid::Uuid::new_v4().to_string(),
            payment_method_data: types::api::PaymentMethod::Card(types::api::CCard {
                card_number: Secret::new("4200000000000000".to_string()),
                card_exp_month: Secret::new("10".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new("999".to_string()),
            }),
            connector_transaction_id: String::new(),
            refund_amount: 100,
        },
        payment_method_id: None,
        response: None,
        error_response: None,
        address: PaymentAddress::default(),
    }
}

#[actix_web::test]

async fn payments_create_success() {
    let conf = Settings::new().unwrap();
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };

    static CV: aci::Aci = aci::Aci;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Aci,
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
}

#[actix_web::test]

async fn payments_create_failure() {
    {
        let conf = Settings::new().unwrap();
        static CV: aci::Aci = aci::Aci;
        let state = AppState {
            flow_name: String::from("default"),
            store: services::Store::new(&conf).await,
            conf,
        };
        let connector = types::api::ConnectorData {
            connector: Box::new(&CV),
            connector_name: types::Connector::Aci,
        };
        let connector_integration: services::BoxedConnectorIntegration<
            types::api::Authorize,
            types::PaymentsRequestData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let mut request = construct_payment_router_data();
        request.request.payment_method_data = types::api::PaymentMethod::Card(types::api::CCard {
            card_number: Secret::new("420000000000000000".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_cvc: Secret::new("99".to_string()),
        });

        let response = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
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
    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
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
    .await
    .unwrap();
    println!("{response:?}");
    assert!(
        response.response.unwrap().refund_status == enums::RefundStatus::Success,
        "The refund transaction failed"
    );
}

#[actix_web::test]

async fn refunds_create_failure() {
    let conf = Settings::new().unwrap();
    static CV: aci::Aci = aci::Aci;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Aci,
    };
    let state = AppState {
        flow_name: String::from("default"),
        store: services::Store::new(&conf).await,
        conf,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        types::api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut request = construct_refund_router_data();
    request.request.connector_transaction_id = "1234".to_string();
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .is_err();
    println!("{response:?}");
    assert!(response, "The refund was intended to fail but it passed");
}
