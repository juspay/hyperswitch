use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken, ConnectorAuthType};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions, PaymentInfo},
};

struct Globalpay;
impl ConnectorActions for Globalpay {}
static CONNECTOR: Globalpay = Globalpay {};
impl Connector for Globalpay {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Globalpay;
        types::api::ConnectorData {
            connector: Box::new(&Globalpay),
            connector_name: types::Connector::Globalpay,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .globalpay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "globalpay".to_string()
    }

    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        Some(json!({"account_name": "transaction_processing"}))
    }
}

fn get_access_token() -> Option<AccessToken> {
    match utils::Connector::get_auth_token(&CONNECTOR) {
        ConnectorAuthType::BodyKey { api_key, key1: _ } => Some(AccessToken {
            token: api_key,
            expires: 18600,
        }),
        _ => None,
    }
}

impl Globalpay {
    fn get_request_interval(&self) -> u64 {
        5
    }
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(types::PaymentAddress {
                billing: Some(api::Address {
                    address: Some(api::AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            access_token: get_access_token(),
            connector_meta_data: CONNECTOR.get_connector_meta(),
            ..Default::default()
        })
    }
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_make_payment() {
    let response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    let x = response.status;
    assert_eq!(x, enums::AttemptStatus::Failure);
}

#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(None, None, Globalpay::get_payment_info())
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(
        CONNECTOR.get_request_interval(),
    ))
    .await;
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            None,
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
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
    let authorize_response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();

    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let refund_response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(
        CONNECTOR.get_request_interval(),
    ))
    .await;
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "You may only refund up to 115% of the original amount ",
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123ddsa12".to_string(), None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Transaction 123ddsa12 not found at this location.")
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Expiry date invalid".to_string(),
    );
}

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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Invalid Expiry Date".to_string(),
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await;
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
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
            Globalpay::get_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
