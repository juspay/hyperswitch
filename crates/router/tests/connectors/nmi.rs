use std::{str::FromStr, time::Duration};

use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct NmiTest;
impl ConnectorActions for NmiTest {}
impl utils::Connector for NmiTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Nmi;
        types::api::ConnectorData {
            connector: Box::new(&Nmi),
            connector_name: types::Connector::Nmi,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nmi
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "nmi".to_string()
    }
}

static CONNECTOR: NmiTest = NmiTest {};

fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            ..utils::CCardType::default().0
        }),
        amount: 2023,
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .expect("Authorize payment response");
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    // Assert the sync response, it will be authorized in case of manual capture, for automatic it will be Completed Success
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(
            transaction_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 1000,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);

    let void_response = CONNECTOR
        .void_payment(
            transaction_id.clone(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("user_cancel".to_string()),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::VoidInitiated);
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Voided,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(
            transaction_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 2023,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id.clone(),
            Some(types::RefundsData {
                refund_amount: 1023,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id.clone(),
            Some(types::RefundsData {
                refund_amount: 1000,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(Duration::from_secs(5)).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                Some(types::RefundsData {
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                None,
            )
            .await
            .unwrap();
        let sync_response = CONNECTOR
            .rsync_retry_till_status_matches(
                enums::RefundStatus::Pending,
                refund_response.response.unwrap().connector_refund_id,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            sync_response.response.unwrap().refund_status,
            enums::RefundStatus::Pending,
        );
    }
}

// Creates a payment with incorrect CVC.
#[ignore = "Connector returns SUCCESS status in case of invalid CVC"]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {}

// Creates a payment with incorrect expiry month.
#[ignore = "Connector returns SUCCESS status in case of expired month."]
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {}

// Creates a payment with incorrect expiry year.
#[ignore = "Connector returns SUCCESS status in case of expired year."]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let void_response = CONNECTOR
        .void_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::VoidFailed);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment("7899353591".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::CaptureFailed);
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id,
            Some(types::RefundsData {
                refund_amount: 3024,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Failure
    );
}
