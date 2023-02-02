use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::{
    connector::Worldline,
    core::errors,
    types::{self, storage::enums, PaymentAddress},
};

use crate::{
    connector_auth::ConnectorAuthentication,
    utils::{self, ConnectorActions, PaymentInfo},
};

struct WorldlineTest;

impl ConnectorActions for WorldlineTest {}
impl utils::Connector for WorldlineTest {
    fn get_data(&self) -> types::api::ConnectorData {
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
        String::from("worldline")
    }
}

impl WorldlineTest {
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some("US".to_string()),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    fn get_payment_authorize_data(
        card_number: &str,
        card_exp_month: &str,
        card_exp_year: &str,
        card_cvc: &str,
        capture_method: enums::CaptureMethod,
    ) -> Option<types::PaymentsAuthorizeData> {
        Some(types::PaymentsAuthorizeData {
            amount: 3500,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethod::Card(types::api::Card {
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
            payment_experience: None,
            payment_issuer: None,
        })
    }
}

#[actix_web::test]
async fn should_requires_manual_authorization() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = WorldlineTest {}
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_auto_authorize_and_request_capture() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = WorldlineTest {}
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

#[actix_web::test]
async fn should_throw_not_implemented_for_unsupported_issuer() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "630495060000000000",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = WorldlineTest {}
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        errors::ConnectorError::NotImplemented(String::from("Payment Method"))
    )
}

#[actix_web::test]
async fn should_throw_missing_required_field_for_country() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = WorldlineTest {}
        .make_payment(
            authorize_data,
            Some(PaymentInfo {
                address: Some(PaymentAddress {
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        errors::ConnectorError::MissingRequiredField {
            field_name: "billing.address.country"
        }
    )
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_cvc() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "25",
        "",
        enums::CaptureMethod::Automatic,
    );
    let response = WorldlineTest {}
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "NULL VALUE NOT ALLOWED FOR cardPaymentMethodSpecificInput.card.cvv".to_string(),
    );
}

#[actix_web::test]
async fn should_sync_manual_auth_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let sync_response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    connector_payment_id,
                ),
                encoded_data: None,
                capture_method: Some(enums::CaptureMethod::Manual),
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_sync_auto_auth_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Pending);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let sync_response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    connector_payment_id,
                ),
                encoded_data: None,
                capture_method: Some(enums::CaptureMethod::Automatic),
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

#[actix_web::test]
async fn should_capture_authorized_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let capture_response = WorldlineTest {}
        .capture_payment(connector_payment_id, None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
}

#[actix_web::test]
async fn should_fail_capture_payment() {
    let capture_response = WorldlineTest {}
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        "UNKNOWN_PAYMENT_ID".to_string()
    );
}

#[actix_web::test]
async fn should_cancel_unauthorized_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let cancel_response = connector
        .void_payment(connector_payment_id, None, None)
        .await
        .unwrap();
    assert_eq!(cancel_response.status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_cancel_uncaptured_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Pending);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let cancel_response = connector
        .void_payment(connector_payment_id, None, None)
        .await
        .unwrap();
    assert_eq!(cancel_response.status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_fail_cancel_with_invalid_payment_id() {
    let response = WorldlineTest {}
        .void_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "UNKNOWN_PAYMENT_ID".to_string(),
    );
}

#[actix_web::test]
async fn should_fail_refund_with_invalid_payment_status() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012000033330026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .make_payment(authorize_data, WorldlineTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response).unwrap_or_default();
    let refund_response = connector
        .refund_payment(connector_payment_id, None, None)
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap_err().message,
        "ORDER WITHOUT REFUNDABLE PAYMENTS".to_string(),
    );
}
