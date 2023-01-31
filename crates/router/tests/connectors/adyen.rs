use masking::Secret;
use router::types::{self, api, storage::enums};
use storage_models::schema::connector_response::encoded_data;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentAuthorizeType},
};
struct Adyen;
impl ConnectorActions for Adyen {}
impl utils::Connector for Adyen {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Adyen;
        types::api::ConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::Connector::Adyen,
            get_token: types::api::GetToken::Connector,
        }
    }
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .adyen
                .expect("Missing connector authentication configuration"),
        )
    }
    fn get_name(&self) -> String {
        "adyen".to_string()
    }
}
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        router_return_url: Some("dfgdf".to_string()),
        address: Some(types::PaymentAddress {
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    first_name: Some(Secret::new("first".to_string())),
                    last_name: Some(Secret::new("last".to_string())),
                    line1: Some(Secret::new("line1".to_string())),
                    line2: Some(Secret::new("line2".to_string())),
                    city: Some("city".to_string()),
                    zip: Some(Secret::new("zip".to_string())),
                    country: Some("IN".to_string()),
                    ..Default::default()
                }),
                phone: Some(api::PhoneDetails {
                    number: Some(Secret::new("1234567890".to_string())),
                    country_code: Some("+91".to_string()),
                }),
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}
fn get_default_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethod::Card(api::CCard {
            card_number: Secret::new("4000020000000000".to_string()),
            card_exp_month: Secret::new("03".to_string()),
            card_exp_year: Secret::new("2030".to_string()),
            card_cvc: Secret::new("737".to_string()),
            ..utils::CCardType::default().0
        }),
        email: Some(Secret::new("abc@gmail.com".to_string())),

        ..PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Adyen {}
        .authorize_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}
#[actix_web::test]
async fn should_make_payment() {
    let response = Adyen {}
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}
#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Adyen {};
    let response = connector
        .authorize_and_capture_payment(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}
#[actix_web::test]
async fn should_partially_capture_already_authorized_payment() {
    let connector = Adyen {};
    let response = connector
        .authorize_and_capture_payment(
            get_default_payment_authorize_data(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: Some(50),
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let connector = Adyen {};
    let authorize_response = connector
        .authorize_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
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
                encoded_data: None,
                capture_method: None,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}
#[actix_web::test]
async fn should_sync_payment() {
    let connector = Adyen {};
    let authorize_response = connector
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
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
                encoded_data: None,
                capture_method: None,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}
#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Adyen {};
    let response = connector
        .authorize_and_void_payment(
            get_default_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(),
                cancellation_reason: Some("requested_by_customer".to_string()),
            }),
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}
#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = Adyen {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4024007134364842".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Refused",);
}
#[actix_web::test]
async fn should_fail_payment_for_no_card_number() {
    let response = Adyen {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Missing payment method details: number",);
}
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = Adyen {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_exp_month: Secret::new("13".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "The provided Expiry Date is not valid.: Expiry month should be between 1 and 12 inclusive: 13",);
}
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_year() {
    let response = Adyen {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_exp_year: Secret::new("2022".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Expired Card",);
}
#[actix_web::test]
async fn should_fail_payment_for_invalid_card_cvc() {
    let response = Adyen {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_cvc: Secret::new("12".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "CVC is not the right length",);
}
// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let connector = Adyen {};
    // Authorize
    let authorize_response = connector
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
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
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded. Only a PaymentIntent with one of the following statuses may be canceled: requires_payment_method, requires_capture, requires_confirmation, requires_action, processing."
    );
}
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector = Adyen {};
    let response = connector
        .capture_payment("12345".to_string(), None, None)
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(
        err.message,
        "Original pspReference required for this operation".to_string()
    );
    assert_eq!(err.code, "167".to_string());
}
#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Adyen {};
    let response = connector
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let connector = Adyen {};
    let response = connector
        .auth_capture_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let connector = Adyen {};
    let refund_response = connector
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
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
    let connector = Adyen {};
    let response = connector
        .auth_capture_and_refund(
            get_default_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
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
    let connector = Adyen {};
    connector
        .make_payment_and_multiple_refund(
            get_default_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}
#[actix_web::test]
async fn should_fail_refund_for_invalid_amount() {
    let connector = Adyen {};
    let response = connector
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
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
    let connector = Adyen {};
    let refund_response = connector
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(),
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
    let connector = Adyen {};
    let refund_response = connector
        .auth_capture_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
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
