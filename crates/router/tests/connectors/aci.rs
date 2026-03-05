use std::str::FromStr;

use hyperswitch_domain_models::{
    address::{Address, AddressDetails, PhoneDetails},
    payment_method_data::{Card, PaymentMethodData},
    router_request_types::AuthenticationData,
};
use masking::Secret;
use router::types::{self, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct AciTest;
impl ConnectorActions for AciTest {}
impl utils::Connector for AciTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Aci;
        utils::construct_connector_data_old(
            Box::new(Aci::new()),
            types::Connector::Aci,
            types::api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .aci
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "aci".to_string()
    }
}

static CONNECTOR: AciTest = AciTest {};

fn get_default_payment_info() -> Option<PaymentInfo> {
    Some(PaymentInfo {
        address: Some(PaymentAddress::new(
            None,
            Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("John".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                    line1: Some(Secret::new("123 Main St".to_string())),
                    city: Some("New York".to_string()),
                    state: Some(Secret::new("NY".to_string())),
                    zip: Some(Secret::new("10001".to_string())),
                    country: Some(enums::CountryAlpha2::US),
                    ..Default::default()
                }),
                phone: Some(PhoneDetails {
                    number: Some(Secret::new("+1234567890".to_string())),
                    country_code: Some("+1".to_string()),
                }),
                email: None,
            }),
            None,
            None,
        )),
        ..Default::default()
    })
}

fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::Card(Card {
            card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("999".to_string()),
            card_holder_name: Some(Secret::new("John Doe".to_string())),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

fn get_threeds_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::Card(Card {
            card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("999".to_string()),
            card_holder_name: Some(Secret::new("John Doe".to_string())),
            ..utils::CCardType::default().0
        }),
        enrolled_for_3ds: true,
        authentication_data: Some(AuthenticationData {
            eci: Some("05".to_string()),
            cavv: Secret::new("jJ81HADVRtXfCBATEp01CJUAAAA".to_string()),
            threeds_server_transaction_id: Some("9458d8d4-f19f-4c28-b5c7-421b1dd2e1aa".to_string()),
            message_version: Some(common_utils::types::SemanticVersion::new(2, 1, 0)),
            ds_trans_id: Some("97267598FAE648F28083B2D2AF7B1234".to_string()),
            created_at: common_utils::date_time::now(),
            challenge_code: Some("01".to_string()),
            challenge_cancel: None,
            challenge_code_reason: Some("01".to_string()),
            message_extension: None,
            acs_trans_id: None,
            authentication_type: None,
            cb_network_params: None,
            exemption_indicator: None,
            transaction_status: None,
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            get_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            get_payment_authorize_data(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            get_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            get_payment_authorize_data(),
            None,
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
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            get_payment_authorize_data(),
            None,
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
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            get_payment_authorize_data(),
            None,
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let response = CONNECTOR
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
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_authorize_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_authorize_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(
            get_payment_authorize_data(),
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
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
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
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            get_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let response = CONNECTOR
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
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(
        response.response.is_err(),
        "Payment should fail with incorrect CVC"
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(
        response.response.is_err(),
        "Payment should fail with invalid expiry month"
    );
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(
        response.response.is_err(),
        "Payment should fail with incorrect expiry year"
    );
}

#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_authorize_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert!(
        void_response.response.is_err(),
        "Void should fail for already captured payment"
    );
}

#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert!(
        capture_response.response.is_err(),
        "Capture should fail for invalid payment ID"
    );
}

#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            get_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(
        response.response.is_err(),
        "Refund should fail when amount exceeds payment amount"
    );
}

#[actix_web::test]
#[ignore]
async fn should_make_threeds_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            get_threeds_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();

    assert!(
        authorize_response.status == enums::AttemptStatus::AuthenticationPending
            || authorize_response.status == enums::AttemptStatus::Charged,
        "3DS payment should result in AuthenticationPending or Charged status, got: {:?}",
        authorize_response.status
    );

    if let Ok(types::PaymentsResponseData::TransactionResponse {
        redirection_data, ..
    }) = &authorize_response.response
    {
        if authorize_response.status == enums::AttemptStatus::AuthenticationPending {
            assert!(
                redirection_data.is_some(),
                "3DS flow should include redirection data for authentication"
            );
        }
    }
}

#[actix_web::test]
#[ignore]
async fn should_authorize_threeds_payment() {
    let response = CONNECTOR
        .authorize_payment(
            get_threeds_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .expect("Authorize 3DS payment response");

    assert!(
        response.status == enums::AttemptStatus::AuthenticationPending
            || response.status == enums::AttemptStatus::Authorized,
        "3DS authorization should result in AuthenticationPending or Authorized status, got: {:?}",
        response.status
    );
}

#[actix_web::test]
#[ignore]
async fn should_sync_threeds_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            get_threeds_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .expect("Authorize 3DS payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::AuthenticationPending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync 3DS response");
    assert!(
        response.status == enums::AttemptStatus::AuthenticationPending
            || response.status == enums::AttemptStatus::Authorized,
        "3DS sync should maintain AuthenticationPending or Authorized status"
    );
}
