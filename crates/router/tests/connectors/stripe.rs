use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct Stripe;
impl ConnectorActions for Stripe {}
impl utils::Connector for Stripe {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Stripe;
        types::api::ConnectorData {
            connector: Box::new(&Stripe),
            connector_name: types::Connector::Stripe,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .stripe
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "stripe".to_string()
    }
}

fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4242424242424242").unwrap(),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Stripe {}
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_make_payment() {
    let response = Stripe {}
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Stripe {};
    let response = connector
        .authorize_and_capture_payment(get_payment_authorize_data(), None, None)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_partially_capture_already_authorized_payment() {
    let connector = Stripe {};
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
    let connector = Stripe {};
    let authorize_response = connector
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
async fn should_sync_payment() {
    let connector = Stripe {};
    let authorize_response = connector
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Stripe {};
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
async fn should_fail_payment_for_incorrect_card_number() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
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
        x.reason.unwrap(),
        "Your card was declined. Your request was in test mode, but used a non test (live) card. For a list of valid test cards, visit: https://stripe.com/docs/testing.",
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("13".to_string()),
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
        x.reason.unwrap(),
        "Your card's expiration month is invalid.",
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_year() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2022".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.reason.unwrap(), "Your card's expiration year is invalid.",);
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_card_cvc() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.reason.unwrap(), "Your card's security code is invalid.",);
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let connector = Stripe {};
    // Authorize
    let authorize_response = connector
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");

    // Void
    let void_response = connector
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().reason.unwrap(),
        "You cannot cancel this PaymentIntent because it has a status of succeeded. Only a PaymentIntent with one of the following statuses may be canceled: requires_payment_method, requires_capture, requires_confirmation, requires_action, processing."
    );
}

#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector = Stripe {};
    let response = connector
        .capture_payment("12345".to_string(), None, None)
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(
        err.reason.unwrap(),
        "No such payment_intent: '12345'".to_string()
    );
    assert_eq!(err.code, "resource_missing".to_string());
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Stripe {};
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
async fn should_refund_manually_captured_payment() {
    let connector = Stripe {};
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
async fn should_partially_refund_succeeded_payment() {
    let connector = Stripe {};
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
async fn should_partially_refund_manually_captured_payment() {
    let connector = Stripe {};
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
async fn should_refund_succeeded_payment_multiple_times() {
    let connector = Stripe {};
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
async fn should_fail_refund_for_invalid_amount() {
    let connector = Stripe {};
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
        response.response.unwrap_err().reason.unwrap(),
        "Refund amount ($1.50) is greater than charge amount ($1.00)",
    );
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = Stripe {};
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
async fn should_sync_manually_captured_refund() {
    let connector = Stripe {};
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
