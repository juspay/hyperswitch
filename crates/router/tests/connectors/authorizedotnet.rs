use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct AuthorizedotnetTest;
impl ConnectorActions for AuthorizedotnetTest {}
impl utils::Connector for AuthorizedotnetTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Authorizedotnet;
        types::api::ConnectorData {
            connector: Box::new(&Authorizedotnet),
            connector_name: types::Connector::Authorizedotnet,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .authorizedotnet
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}
static CONNECTOR: AuthorizedotnetTest = AuthorizedotnetTest {};

fn get_payment_method_data() -> api::Card {
    api::Card {
        card_number: cards::CardNumber::from_str("5424000000000015").unwrap(),
        card_exp_month: Secret::new("02".to_string()),
        card_exp_year: Secret::new("2035".to_string()),
        card_holder_name: Secret::new("John Doe".to_string()),
        card_cvc: Secret::new("123".to_string()),
        ..Default::default()
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 300,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");

    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 301,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let cap_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 301,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 302,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let cap_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 150,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 303,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).x
#[actix_web::test]
async fn should_void_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 304,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");

    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let void_response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                amount: Some(304),
                ..utils::PaymentCancelType::default().0
            }),
            None,
        )
        .await
        .expect("Void response");
    assert_eq!(void_response.status, enums::AttemptStatus::VoidInitiated)
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let cap_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 310,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let txn_id = utils::get_connector_transaction_id(cap_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(
        psync_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 311,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_refund() {
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "60217566768".to_string(),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates a payment with empty card number.
#[actix_web::test]
async fn should_fail_payment_for_empty_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardNumber' element is invalid - The value XX is invalid according to its datatype 'String' - The actual length is less than the MinLength value.",
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardCode' element is invalid - The value XXXXXXX is invalid according to its datatype 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardCode' - The actual length is greater than the MaxLength value.".to_string(),
    );
}
// todo()

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Credit card expiration date is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The credit card has expired.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 307,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:amount' element is invalid - The value &#39;&#39; is invalid according to its datatype 'http://www.w3.org/2001/XMLSchema:decimal' - The string &#39;&#39; is not a valid Decimal value."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        "The transaction cannot be found."
    );
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_partially_refund_manually_captured_payment() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_refund_manually_captured_payment() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_sync_manually_captured_refund() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_refund_auto_captured_payment() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_partially_refund_succeeded_payment() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_refund_succeeded_payment_multiple_times() {}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
