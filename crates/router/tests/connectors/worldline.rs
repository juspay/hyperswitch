use std::str::FromStr;

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
        /// This method returns a ConnectorData object containing information about the connector,
    /// including the connector type, connector name, token retrieval method, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        types::api::ConnectorData {
            connector: Box::new(&Worldline),
            connector_name: types::Connector::Worldline,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It first creates a new instance of ConnectorAuthentication and then converts it to a ConnectorAuthType using the to_connector_auth_type function from the utils module. If the worldline field in the ConnectorAuthentication instance is missing, it will panic with the message "Missing connector authentication configuration".
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            ConnectorAuthentication::new()
                .worldline
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "worldline" as a String.
    fn get_name(&self) -> String {
        String::from("worldline")
    }
}

impl WorldlineTest {
        /// Retrieves the payment information, if available.
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

        /// Constructs and returns the payment authorization data for a given card, including the card details, amount, currency, and other relevant information.
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
            payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
                card_number: cards::CardNumber::from_str(card_number).unwrap(),
                card_exp_month: Secret::new(card_exp_month.to_string()),
                card_exp_year: Secret::new(card_exp_year.to_string()),
                card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
                card_cvc: Secret::new(card_cvc.to_string()),
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
            capture_method: Some(capture_method),
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            customer_name: None,
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
            request_incremental_authorization: false,
            metadata: None,
        })
    }
}

#[actix_web::test]
/// Asynchronously performs a payment authorization with manual authorization required.
async fn should_requires_manual_authorization() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "5424 1802 7979 1732",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = WorldlineTest {}
        .authorize_payment(authorize_data, WorldlineTest::get_payment_info())
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
/// Asynchronously performs payment authorization and requests automatic capture using the Worldline API.
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
/// Asynchronously executes a payment authorization using test data and expects the method to throw a `NotSupported` error for an unsupported issuer.
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
        errors::ConnectorError::NotSupported {
            message: "Maestro".to_string(),
            connector: "worldline",
        }
    )
}

#[actix_web::test]
/// This asynchronous method tests for the scenario where a required field for the country in the billing address is missing. It creates payment authorization data with specific details, makes a payment with missing country field in the billing address, and asserts that the response contains an error indicating the missing required field for the country in the billing address.
async fn should_throw_missing_required_field_for_country() {
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012 0000 3333 0026",
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
/// This method tests the payment system's behavior when a payment with an invalid CVC (Card Verification Code) is attempted. It creates payment authorization data with a specific card number, expiration date, empty CVC, and automatic capture method, then attempts to make the payment using the created data and payment information. It then asserts that the response contains an error message indicating that a null value is not allowed for the card's CVC.
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
/// Asynchronously initiates a manual authorization payment sync process by creating an instance of WorldlineTest, retrieving payment authorization data, authorizing the payment, and synchronizing the payment. It then asserts that the response status is "Authorized" for both authorization and synchronization processes.
async fn should_sync_manual_auth_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012 0000 3333 0026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .authorize_payment(authorize_data, WorldlineTest::get_payment_info())
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
                capture_method: Some(enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
/// Asynchronously performs an auto-auth payment synchronization with the Worldline connector. 
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
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

#[actix_web::test]
/// Asynchronously captures an authorized payment using WorldlineTest connector.
async fn should_capture_authorized_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012 0000 3333 0026",
        "10",
        "2025",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .authorize_payment(authorize_data, WorldlineTest::get_payment_info())
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
/// Asynchronously attempts to capture a payment using the WorldlineTest API and expects the capture to fail with an "UNKNOWN_PAYMENT_ID" error message.
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
/// This method is used to cancel an unauthorized payment by performing the following steps:
/// 1. Initialize a WorldlineTest connector.
/// 2. Retrieve payment authorization data.
/// 3. Authorize the payment using the connector and the authorization data.
/// 4. Verify that the payment was successfully authorized.
/// 5. Retrieve the connector payment ID from the authorization response.
/// 6. Void the payment using the connector and the connector payment ID.
/// 7. Verify that the payment was successfully voided.
async fn should_cancel_unauthorized_payment() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012 0000 3333 0026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .authorize_payment(authorize_data, WorldlineTest::get_payment_info())
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
/// Asynchronously cancels an uncaptured payment by creating an authorize request, making the payment, and then voiding the payment.
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
/// Asynchronously tests if the void_payment method fails with an invalid payment ID by attempting to void a payment with an invalid payment ID and verifying that the response contains the expected error message "UNKNOWN_PAYMENT_ID".
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
/// Attempts to refund a payment with an invalid payment status and expects the refund to fail with a specific error message. 
async fn should_fail_refund_with_invalid_payment_status() {
    let connector = WorldlineTest {};
    let authorize_data = WorldlineTest::get_payment_authorize_data(
        "4012 0000 3333 0026",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Manual,
    );
    let response = connector
        .authorize_payment(authorize_data, WorldlineTest::get_payment_info())
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
