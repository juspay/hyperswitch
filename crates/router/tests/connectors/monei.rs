use std::str::FromStr;

use masking::Secret;
use router::types::{self, domain, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct Monei;
impl ConnectorActions for Monei {}
impl utils::Connector for Monei {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Monei;
        utils::construct_connector_data_old(
            Box::new(Monei::new()),
            types::Connector::Monei,
            types::api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .monei
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "monei".to_string()
    }
}

fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: domain::PaymentMethodData::Card(domain::Card {
            card_number: cards::CardNumber::from_str("4242424242424242").unwrap(),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Monei {}
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_make_payment() {
    let response = Monei {}
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_authorized_payment() {
    let connector = Monei {};
    let response = connector
        .authorize_and_capture_payment(get_payment_authorize_data(), None, None)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let connector = Monei {};
    let response = connector
        .authorize_and_capture_payment(
            get_payment_authorize_data(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_authorized_payment() {
    let connector = Monei {};
    let authorize_response = connector
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_void_authorized_payment() {
    let connector = Monei {};
    let response = connector
        .authorize_and_void_payment(
            get_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(), // this connector_transaction_id will be ignored and the transaction_id from payment authorize data will be used for void
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let connector = Monei {};
    let authorize_response = connector
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let connector = Monei {};
    let response = connector
        .auth_capture_and_refund(get_payment_authorize_data(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let connector = Monei {};
    let response = connector
        .auth_capture_and_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let connector = Monei {};
    let refund_response = connector
        .auth_capture_and_refund(get_payment_authorize_data(), None, None)
        .await
        .unwrap();
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
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

#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let connector = Monei {};
    let response = connector
        .make_payment_and_refund(get_payment_authorize_data(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let connector = Monei {};
    let refund_response = connector
        .make_payment_and_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let connector = Monei {};
    connector
        .make_payment_and_multiple_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await;
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = Monei {};
    let refund_response = connector
        .make_payment_and_refund(get_payment_authorize_data(), None, None)
        .await
        .unwrap();
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
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

#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = Monei {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: domain::PaymentMethodData::Card(domain::Card {
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
        "Your card's security code is invalid.".to_string(),
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = Monei {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: domain::PaymentMethodData::Card(domain::Card {
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
        "Your card's expiration month is invalid.".to_string(),
    );
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = Monei {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: domain::PaymentMethodData::Card(domain::Card {
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
        "Your card's expiration year is invalid.".to_string(),
    );
}

#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let connector = Monei {};
    let authorize_response = connector
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = connector
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector = Monei {};
    let response = connector
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        String::from("No such payment_intent: '123456789'")
    );
}

#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let connector = Monei {};
    let response = connector
        .make_payment_and_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Refund amount (₹1.50) is greater than charge amount (₹1.00)",
    );
}
