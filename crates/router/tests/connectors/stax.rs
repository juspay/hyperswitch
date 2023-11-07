use std::{str::FromStr, time::Duration};

use masking::Secret;
use router::types::{self, api, storage::enums, PaymentsResponseData};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct StaxTest;
impl ConnectorActions for StaxTest {}
impl utils::Connector for StaxTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Stax;
        types::api::ConnectorData {
            connector: Box::new(&Stax),
            connector_name: types::Connector::Stax,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .stax
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "stax".to_string()
    }
}

static CONNECTOR: StaxTest = StaxTest {};

fn get_default_payment_info(
    connector_customer: Option<String>,
    payment_method_token: Option<String>,
) -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: None,
        auth_type: None,
        access_token: None,
        connector_meta_data: None,
        return_url: None,
        connector_customer,
        payment_method_token,
        payout_method_data: None,
        currency: None,
        country: None,
    })
}

fn customer_details() -> Option<types::ConnectorCustomerData> {
    Some(types::ConnectorCustomerData {
        ..utils::CustomerType::default().0
    })
}

fn token_details() -> Option<types::PaymentMethodTokenizationData> {
    Some(types::PaymentMethodTokenizationData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            card_exp_month: Secret::new("04".to_string()),
            card_exp_year: Secret::new("2027".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: None,
        amount: None,
        currency: enums::Currency::USD,
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        ..utils::PaymentAuthorizeType::default().0
    })
}

async fn create_customer_and_get_token() -> Option<String> {
    let customer_response = CONNECTOR
        .create_connector_customer(customer_details(), get_default_payment_info(None, None))
        .await
        .expect("Authorize payment response");
    let connector_customer_id = match customer_response.response.unwrap() {
        PaymentsResponseData::ConnectorCustomerResponse {
            connector_customer_id,
        } => Some(connector_customer_id),
        _ => None,
    };

    let token_response = CONNECTOR
        .create_connector_pm_token(
            token_details(),
            get_default_payment_info(connector_customer_id, None),
        )
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
            get_default_payment_info(None, create_customer_and_get_token().await),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(None, create_customer_and_get_token().await),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(None, None),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let capture_response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Capture payment response");

    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_meta = utils::get_connector_metadata(capture_response.response);

    let response = CONNECTOR
        .refund_payment(
            refund_txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(None, None),
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
    let capture_response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Capture payment response");

    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_meta = utils::get_connector_metadata(capture_response.response);

    let response = CONNECTOR
        .refund_payment(
            refund_txn_id,
            Some(types::RefundsData {
                refund_amount: 50,
                connector_metadata: refund_connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(None, None),
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
    let capture_response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .expect("Capture payment response");

    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_meta = utils::get_connector_metadata(capture_response.response);

    let refund_response = CONNECTOR
        .refund_payment(
            refund_txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(None, None),
        )
        .await
        .unwrap();

    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None, None),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
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
            get_default_payment_info(None, None),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            None,
            get_default_payment_info(None, create_customer_and_get_token().await),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let payment_method_token = create_customer_and_get_token().await;

    let response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(None, payment_method_token.clone()),
        )
        .await
        .unwrap();

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(Duration::from_secs(60)).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                Some(types::RefundsData {
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                get_default_payment_info(None, payment_method_token.clone()),
            )
            .await
            .unwrap();
        assert_eq!(
            refund_response.response.unwrap().refund_status,
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None, None),
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
    let customer_response = CONNECTOR
        .create_connector_customer(customer_details(), get_default_payment_info(None, None))
        .await
        .expect("Authorize payment response");
    let connector_customer_id = match customer_response.response.unwrap() {
        PaymentsResponseData::ConnectorCustomerResponse {
            connector_customer_id,
        } => Some(connector_customer_id),
        _ => None,
    };

    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("11".to_string()),
                    card_exp_year: Secret::new("2027".to_string()),
                    card_cvc: Secret::new("123456".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: enums::Currency::USD,
            }),
            get_default_payment_info(connector_customer_id, None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response.response.unwrap_err().reason,
        Some(r#"{"card_cvv":["The card cvv may not be greater than 99999."]}"#.to_string()),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let customer_response = CONNECTOR
        .create_connector_customer(customer_details(), get_default_payment_info(None, None))
        .await
        .expect("Authorize payment response");
    let connector_customer_id = match customer_response.response.unwrap() {
        PaymentsResponseData::ConnectorCustomerResponse {
            connector_customer_id,
        } => Some(connector_customer_id),
        _ => None,
    };

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
                currency: enums::Currency::USD,
            }),
            get_default_payment_info(connector_customer_id, None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response.response.unwrap_err().reason,
        Some(r#"{"validation":["Tokenization Validation Errors: Month is invalid"]}"#.to_string()),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let customer_response = CONNECTOR
        .create_connector_customer(customer_details(), get_default_payment_info(None, None))
        .await
        .expect("Authorize payment response");
    let connector_customer_id = match customer_response.response.unwrap() {
        PaymentsResponseData::ConnectorCustomerResponse {
            connector_customer_id,
        } => Some(connector_customer_id),
        _ => None,
    };

    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("04".to_string()),
                    card_exp_year: Secret::new("2000".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: enums::Currency::USD,
            }),
            get_default_payment_info(connector_customer_id, None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response.response.unwrap_err().reason,
        Some(r#"{"validation":["Tokenization Validation Errors: Year is invalid"]}"#.to_string()),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector Refunds the payment on Void call for Auto Captured Payment"]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info(None, None))
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment(
            "123456789".to_string(),
            None,
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().reason,
        Some(r#"{"id":["The selected id is invalid."]}"#.to_string()),
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
            get_default_payment_info(None, create_customer_and_get_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().reason,
        Some(r#"{"total":["The total may not be greater than 1."]}"#.to_string()),
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
