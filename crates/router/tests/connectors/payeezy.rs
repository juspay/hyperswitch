use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::{
    core::errors,
    types::{self, api, storage::enums},
};

use crate::{
    connector_auth::{self},
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct PayeezyTest;
impl ConnectorActions for PayeezyTest {}
static CONNECTOR: PayeezyTest = PayeezyTest {};
impl utils::Connector for PayeezyTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payeezy;
        types::api::ConnectorData {
            connector: Box::new(&Payeezy),
            connector_name: types::Connector::Payeezy,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .payeezy
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "payeezy".to_string()
    }
}

impl PayeezyTest {
    fn get_payment_data() -> Option<types::PaymentsAuthorizeData> {
        Some(types::PaymentsAuthorizeData {
            payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                card_number: Secret::new(String::from("4012000033330026")),
                ..utils::CCardType::default().0
            }),
            ..utils::PaymentAuthorizeType::default().0
        })
    }

    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(types::PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some("US".to_string()),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            router_return_url: Some(String::from("http://localhost:8080")),
            ..Default::default()
        })
    }
    fn get_request_interval(&self) -> u64 {
        20
    }
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
                card_number: Secret::new(card_number.to_string()),
                card_exp_month: Secret::new(card_exp_month.to_string()),
                card_exp_year: Secret::new(card_exp_year.to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new(card_cvc.to_string()),
                card_issuer: None,
                card_network: None,
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
            email: None,
            payment_experience: None,
            payment_method_type: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
        })
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = PayeezyTest {}
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = PayeezyTest {}
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_meta(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta: connector_meta,
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = PayeezyTest {}
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = PayeezyTest {}
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_meta(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta: connector_meta,
        amount_to_capture: Some(50),
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = PayeezyTest {}
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, None)
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
                encoded_data: None,
                capture_method: None,
                connector_meta: None,
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = PayeezyTest {}
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_meta(response.response);
    tokio::time::sleep(std::time::Duration::from_secs(
        PayeezyTest {}.get_request_interval(),
    ))
    .await; // to avoid 404 error
    let response = PayeezyTest {}
        .void_payment(
            connector_payment_id,
            Some(types::PaymentsCancelData {
                connector_meta: connector_meta,
                amount: Some(100),
                currency: Some(storage_models::enums::Currency::USD),
                ..utils::PaymentCancelType::default().0
            }),
            None,
        )
        .await
        .unwrap();

    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 50,
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
#[ignore]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, None)
        .await
        .unwrap();
    let response = CONNECTOR
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
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
                encoded_data: None,
                capture_method: Some(enums::CaptureMethod::Automatic),
                connector_meta: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 50,
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_transaction_id: txn_id.clone(),
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response.clone());
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                capture_txn_id.clone(),
                Some(types::RefundsData {
                    connector_metadata: refund_connector_metadata.clone(),
                    connector_transaction_id: capture_txn_id.clone(),
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                PayeezyTest::get_payment_info(),
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
#[ignore]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    let response = CONNECTOR
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

// Cards Negative scenerios
// Creates a payment with incorrect card issuer.

#[actix_web::test]
async fn should_throw_not_implemented_for_unsupported_issuer() {
    let authorize_data = PayeezyTest::get_payment_authorize_data(
        "630495060000000000",
        "10",
        "25",
        "123",
        enums::CaptureMethod::Automatic,
    );
    let response = PayeezyTest {}
        .make_payment(authorize_data, PayeezyTest::get_payment_info())
        .await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        errors::ConnectorError::NotSupported {
            payment_method: "card".to_string(),
            connector: "Payeezy",
            payment_experience: "RedirectToUrl".to_string(),
        }
    )
}

// Creates a payment with empty card number.
#[actix_web::test]
async fn should_fail_payment_for_empty_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new(String::from("")),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        errors::ConnectorError::NotImplemented("Card Type".to_string())
    )
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345d".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        *response.response.unwrap_err().message,
        "The cvv provided must be numeric".to_string(),
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        *response.response.unwrap_err().message,
        "Bad Request (25) - Invalid Expiry Date".to_string(),
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
        "Expiry Date is invalid".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(capture_response.clone().response);
    let connector_meta = utils::get_connector_meta(capture_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(
            txn_id.unwrap(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                connector_meta: connector_meta,
                ..Default::default()
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(
        void_response.response.unwrap_err().message,
        "Bad Request (26) - Invalid Amount"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let response = PayeezyTest {}
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "123455678".to_string();
    let connector_meta = utils::get_connector_meta(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta: connector_meta,
        amount_to_capture: Some(50),
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = PayeezyTest {}
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Bad Request (69) - Invalid Transaction Tag")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_meta(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_meta(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 1500,
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        String::from("Bad Request (64) - Invalid Refund"),
    );
}
