use std::{str::FromStr, time::Duration};

use masking::Secret;
use router::types::{
    self, api,
    storage::{self, enums},
    PaymentsResponseData,
};
use test_utils::connector_auth::ConnectorAuthentication;

use crate::utils::{self, get_connector_transaction_id, Connector, ConnectorActions};

#[derive(Clone, Copy)]
struct SquareTest;
impl ConnectorActions for SquareTest {}
impl Connector for SquareTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Square;
        types::api::ConnectorData {
            connector: Box::new(&Square),
            connector_name: types::Connector::Square,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            ConnectorAuthentication::new()
                .square
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "square".to_string()
    }
}

static CONNECTOR: SquareTest = SquareTest {};

fn get_default_payment_info(payment_method_token: Option<String>) -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: None,
        auth_type: None,
        access_token: None,
        connector_meta_data: None,
        return_url: None,
        connector_customer: None,
        payment_method_token,
        payout_method_data: None,
        currency: None,
        country: None,
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

fn token_details() -> Option<types::PaymentMethodTokenizationData> {
    Some(types::PaymentMethodTokenizationData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            card_exp_month: Secret::new("04".to_string()),
            card_exp_year: Secret::new("2027".to_string()),
            card_cvc: Secret::new("100".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: None,
        amount: None,
        currency: storage::enums::Currency::USD,
    })
}

async fn create_token() -> Option<String> {
    let token_response = CONNECTOR
        .create_connector_pm_token(token_details(), get_default_payment_info(None))
        .await
        .expect("Authorize payment response");
    match token_response.response.unwrap() {
        PaymentsResponseData::TokenizationResponse { token } => Some(token),
        _ => None,
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support partial capture"]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    //make a successful payment
    let response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let refund_data = Some(types::RefundsData {
        refund_amount: 50,
        ..utils::PaymentRefundType::default().0
    });
    //try refund for previous payment
    let transaction_id = get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(Duration::from_secs(CONNECTOR.get_request_interval())).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                refund_data.clone(),
                get_default_payment_info(None),
            )
            .await
            .unwrap();
        let response = CONNECTOR
            .rsync_retry_till_status_matches(
                enums::RefundStatus::Success,
                refund_response.response.unwrap().connector_refund_id,
                None,
                get_default_payment_info(None),
            )
            .await
            .unwrap();
        assert_eq!(
            response.response.unwrap().refund_status,
            enums::RefundStatus::Success,
        );
    }
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Cards Negative scenerios
// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("11".to_string()),
                    card_exp_year: Secret::new("2027".to_string()),
                    card_cvc: Secret::new("".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Missing required parameter.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("20".to_string()),
                    card_exp_year: Secret::new("2027".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Invalid card expiration date.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("11".to_string()),
                    card_exp_year: Secret::new("2000".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Invalid card expiration date.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(
            txn_id.clone().unwrap(),
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    let connector_transaction_id = txn_id.unwrap();
    assert_eq!(
        void_response.response.unwrap_err().reason.unwrap_or("".to_string()),
        format!("Payment {connector_transaction_id} is in inflight state COMPLETED, which is invalid for the requested operation")
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment(
            "123456789".to_string(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        String::from("Could not find payment with id: 123456789")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "The requested refund amount exceeds the amount available to refund.",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
