use std::marker::PhantomData;

use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::{
    configs::settings::Settings,
    core::payments,
    db::StorageImpl,
    routes, services,
    types::{self, storage::enums, PaymentAddress, PaymentsAuthorizeRouterData},
};

use crate::{connector_auth::ConnectorAuthentication, utils, utils::ConnectorActions};

struct Worldline;

impl ConnectorActions for Worldline {}
impl utils::Connector for Worldline {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Worldline;
        types::api::ConnectorData {
            connector: Box::new(&Worldline),
            connector_name: types::Connector::Worldline,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            ConnectorAuthentication::new()
                .worldline
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "worldline".to_string()
    }
}

impl Worldline {
    fn construct_payment_router_data(
        request: types::PaymentsAuthorizeData,
    ) -> PaymentsAuthorizeRouterData {
        let auth = ConnectorAuthentication::new()
            .worldline
            .expect("Missing worldline connector authentication configuration");

        types::RouterData {
            flow: PhantomData,
            merchant_id: String::from("worldline"),
            connector: "worldline".to_string(),
            payment_id: uuid::Uuid::new_v4().to_string(),
            status: enums::AttemptStatus::default(),
            return_url: None,
            payment_method: enums::PaymentMethodType::Card,
            connector_auth_type: auth.into(),
            auth_type: enums::AuthenticationType::NoThreeDs,
            description: Some("This is a test".to_string()),
            router_return_url: None,
            request,
            payment_method_id: None,
            response: Err(types::ErrorResponse::default()),
            address: PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some("US".to_string()),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            },
            connector_meta_data: None,
            amount_captured: None,
            attempt_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    fn get_payment_authorize_data(
        card_number: &str,
        card_exp_month: &str,
        card_exp_year: &str,
        card_cvc: &str,
        capture_method: enums::CaptureMethod,
    ) -> types::PaymentsAuthorizeData {
        types::PaymentsAuthorizeData {
            amount: 3500,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethod::Card(types::api::CCard {
                card_number: Secret::new(card_number.to_string()),
                card_exp_month: Secret::new(card_exp_month.to_string()),
                card_exp_year: Secret::new(card_exp_year.to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new(card_cvc.to_string()),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: Some(capture_method),
            browser_info: None,
            order_details: None,
            email: None,
        }
    }
}

#[actix_web::test]
async fn should_requires_manual_authorization() {
    let conf = Settings::new().unwrap();
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    static CV: router::connector::Worldline = router::connector::Worldline;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Worldline,
        get_token: types::api::GetToken::Connector,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = Worldline::construct_payment_router_data(Worldline::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Manual,
    ));

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
        response.status == enums::AttemptStatus::Authorizing,
        "The payment failed"
    );
}

#[actix_web::test]
async fn should_auto_authorize_and_request_capture() {
    let conf = Settings::new().unwrap();
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    static CV: router::connector::Worldline = router::connector::Worldline;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Worldline,
        get_token: types::api::GetToken::Connector,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = Worldline::construct_payment_router_data(Worldline::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Automatic,
    ));

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
        response.status == enums::AttemptStatus::CaptureInitiated,
        "The payment failed"
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_cvc() {
    let conf = Settings::new().unwrap();
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    static CV: router::connector::Worldline = router::connector::Worldline;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Worldline,
        get_token: types::api::GetToken::Connector,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = Worldline::construct_payment_router_data(Worldline::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "",
        enums::CaptureMethod::Automatic,
    ));

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();
    println!("{response:?}");
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "NULL VALUE NOT ALLOWED FOR cardPaymentMethodSpecificInput.card.cvv".to_string(),
    );
}
