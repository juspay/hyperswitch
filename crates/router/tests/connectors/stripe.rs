use std::time::Duration;

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
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .stripe
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "stripe".to_string()
    }
}

fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethod::Card(api::CCard {
            card_number: Secret::new("4242424242424242".to_string()),
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
async fn should_authorize_and_capture_payment() {
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
                amount_to_capture: Some(50),
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = Stripe {};
    let authorize_response = connector
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Stripe {};
    let response = connector
        .authorize_and_void_payment(
            get_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(),
                cancellation_reason: Some("requested_by_customer".to_string()),
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
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4024007134364842".to_string()),
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
        "Your card was declined. Your request was in test mode, but used a non test (live) card. For a list of valid test cards, visit: https://stripe.com/docs/testing.",
    );
}

#[actix_web::test]
async fn should_fail_payment_for_no_card_number() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("".to_string()),
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
        "You passed an empty string for 'payment_method_data[card][number]'. We assume empty values are an attempt to unset a parameter; however 'payment_method_data[card][number]' cannot be unset. You should remove 'payment_method_data[card][number]' from your request or supply a non-empty value.",
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
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
    assert_eq!(x.message, "Your card's expiration month is invalid.",);
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_year() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
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
    assert_eq!(x.message, "Your card's expiration year is invalid.",);
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_card_cvc() {
    let response = Stripe {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
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
    assert_eq!(x.message, "Your card's security code is invalid.",);
}

#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector = Stripe {};
    let authorize_response = connector
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    tokio::time::sleep(Duration::from_secs(5)).await; // to avoid 404 error as stripe takes some time to process the new transaction
    let response = connector
        .capture_payment("12345".to_string(), None, None)
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(err.message, "No such payment_intent: '12345'".to_string());
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
        response.response.unwrap_err().message,
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
