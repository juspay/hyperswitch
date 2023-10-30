use std::str::FromStr;

use cards::CardNumber;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentsAuthorizeData};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct NexinetsTest;
impl ConnectorActions for NexinetsTest {}
static CONNECTOR: NexinetsTest = NexinetsTest {};
impl utils::Connector for NexinetsTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Nexinets;
        types::api::ConnectorData {
            connector: Box::new(&Nexinets),
            connector_name: types::Connector::Nexinets,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nexinets
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "nexinets".to_string()
    }
}

fn payment_method_details() -> Option<PaymentsAuthorizeData> {
    Some(PaymentsAuthorizeData {
        currency: diesel_models::enums::Currency::EUR,
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: CardNumber::from_str("374111111111111").unwrap(),
            ..utils::CCardType::default().0
        }),
        router_return_url: Some("https://google.com".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}
// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
        currency: diesel_models::enums::Currency::EUR,
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = CONNECTOR
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
        amount_to_capture: 50,
        currency: diesel_models::enums::Currency::EUR,
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = CONNECTOR
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta,
                mandate_id: None,
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
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let response = CONNECTOR
        .void_payment(
            connector_payment_id,
            Some(types::PaymentsCancelData {
                connector_meta,
                amount: Some(100),
                currency: Some(diesel_models::enums::Currency::EUR),
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
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                connector_transaction_id: capture_txn_id,
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: refund_connector_metadata,
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

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 10,
                connector_transaction_id: capture_txn_id,
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: refund_connector_metadata,
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let refund_response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 100,
                connector_transaction_id: capture_txn_id.clone(),
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: refund_connector_metadata.clone(),
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let transaction_id = Some(
        refund_response
            .response
            .clone()
            .unwrap()
            .connector_refund_id,
    );
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response
                .response
                .clone()
                .unwrap()
                .connector_refund_id,
            Some(types::RefundsData {
                connector_refund_id: transaction_id,
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let cap_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(cap_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(cap_response.response.clone()).unwrap();
    let connector_meta = utils::get_connector_metadata(cap_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                capture_method: Some(enums::CaptureMethod::Automatic),
                connector_meta,
                ..Default::default()
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
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_metadata = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 100,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata,
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

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 50,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
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

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                txn_id.clone().unwrap(),
                Some(types::RefundsData {
                    connector_metadata: connector_meta.clone(),
                    connector_transaction_id: txn_id.clone().unwrap(),
                    refund_amount: 50,
                    currency: diesel_models::enums::Currency::EUR,
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
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_refund() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_metadata = utils::get_connector_metadata(captured_response.response).clone();
    let refund_response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                connector_transaction_id: txn_id.clone().unwrap(),
                refund_amount: 100,
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: connector_metadata.clone(),
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let transaction_id = Some(
        refund_response
            .response
            .clone()
            .unwrap()
            .connector_refund_id,
    );
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response
                .response
                .clone()
                .unwrap()
                .connector_refund_id,
            Some(types::RefundsData {
                connector_refund_id: transaction_id,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata,
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

// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.verification : Bad value for 'payment.verification'. Expected: string of length in range 3 <=> 4 representing a valid creditcard verification number.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.expiryMonth : Bad value for 'payment.expiryMonth'. Expected: string of length 2 in range '01' <=> '12' representing the month in a valid creditcard expiry date >= current date.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.expiryYear : Bad value for 'payment.expiryYear'. Expected: string of length 2 in range '01' <=> '99' representing the year in a valid creditcard expiry date >= current date.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone()).unwrap();
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let void_response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                cancellation_reason: Some("requested_by_customer".to_string()),
                amount: Some(100),
                currency: Some(diesel_models::enums::Currency::EUR),
                connector_meta,
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "transactionId : Operation not allowed!"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector_payment_id = "".to_string();
    let capture_response = CONNECTOR
        .capture_payment(
            connector_payment_id,
            Some(types::PaymentsCaptureData {
                connector_meta: Some(
                    serde_json::json!({"transaction_id" : "transaction_usmh41hymb",
                        "order_id" : "tjil1ymxsz",
                        "psync_flow" : "PREAUTH"
                    }),
                ),
                amount_to_capture: 50,
                currency: diesel_models::enums::Currency::EUR,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("transactionId : Transaction does not belong to order.")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 150,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "initialAmount : Bad value for 'initialAmount'. Expected: Positive integer between 1 and maximum available amount (debit/capture.initialAmount - debit/capture.refundedAmount.",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
