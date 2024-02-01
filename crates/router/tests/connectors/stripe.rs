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
        /// Retrieves the connector data for the Stripe connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Stripe;
        types::api::ConnectorData {
            connector: Box::new(&Stripe),
            connector_name: types::Connector::Stripe,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .stripe
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "stripe" as a String.
    fn get_name(&self) -> String {
        "stripe".to_string()
    }
}

/// Retrieves the payment authorization data for a payment method, if available.
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
/// Asynchronously sends a request to the Stripe API to authorize a payment using the provided payment authorization data. 
/// It then asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = Stripe {}
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
/// Asynchronously makes a payment using the Stripe API with the provided payment authorize data.
async fn should_make_payment() {
    let response = Stripe {}
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously captures an already authorized payment using the Stripe connector. 
async fn should_capture_already_authorized_payment() {
    let connector = Stripe {};
    let response = connector
        .authorize_and_capture_payment(get_payment_authorize_data(), None, None)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously authorizes and partially captures a payment that has already been authorized,
/// using the Stripe connector. It uses the payment authorization data retrieved from
/// get_payment_authorize_data() and captures a specified amount. It then checks if the
/// response status is charged and asserts the equality.
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
/// Asynchronously initiates the process of synchronizing an authorized payment with the Stripe connector. 
/// This method first authorizes the payment with the Stripe connector, retrieves the transaction ID, 
/// and then retries the synchronization process until the status of the payment matches the authorized status. 
/// Finally, it asserts that the response status matches the authorized status.
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
/// Asynchronously checks if a payment should be synced with the Stripe connector. It makes a payment authorization request to the Stripe connector, retrieves the transaction ID, and then retries syncing the payment until the status matches the 'Charged' status. It asserts that the final response status is 'Charged'.
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
/// Asynchronously attempts to void an already authorized payment using the Stripe connector. 
/// If successful, it will return the voided status of the payment.
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
/// Makes a payment using an incorrect card number and expects the payment to fail with a specific error message.
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
/// This asynchronous method simulates a payment attempt using an invalid expiration month for a card. It constructs a `PaymentAuthorizeData` object with a card data having an expiration month of "13" and uses the `make_payment` method of the `Stripe` struct to make the payment. It then asserts that the response contains an error with the reason "Your card's expiration month is invalid."
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
/// Asynchronously tests that payment fails for an invalid expiration year by making a payment using the Stripe API with invalid expiration year data and asserting that the response contains an error with the expected reason.
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
/// Asynchronously makes a payment using Stripe API with invalid card CVC, and asserts that the payment fails with the expected error message.
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
/// Asynchronously attempts to void a payment that has been auto-captured, and asserts that the void operation fails with the appropriate error message.
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
/// Asynchronously captures a payment using the Stripe connector and checks for an invalid payment error.
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
/// Asynchronously makes a payment and then attempts to refund it. The method uses the Stripe connector
/// to make the payment and then waits for the response. If the refund is successful, the method will assert
/// that the refund status is "Success".
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
/// Asynchronously calls the Stripe API to manually refund a payment that has been previously captured and authorized. It uses the `auth_capture_and_refund` method of the Stripe connector to capture the payment, and then awaits the response to ensure the refund is successful. It then asserts that the refund status is 'Success' using the `enums::RefundStatus` enum.
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
/// Asynchronously makes a partial refund for a succeeded payment using the Stripe connector. It first creates a payment and then issues a refund for a specified refund amount. The method expects the payment authorization data, refund amount, and optional refund type. It then awaits the refund response and asserts that the refund status is a success.
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
/// Asynchronously performs a partial refund for a manually captured payment using the Stripe connector. 
/// It first captures the authorized payment and then refunds a specified amount. 
/// If the refund is successful, it returns a response with the refund status indicating success.
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
/// Asynchronously makes a payment and then attempts to refund the payment multiple times using the Stripe connector. It uses the payment authorization data obtained from get_payment_authorize_data() and specifies a refund amount of 50. If no refund type is specified, it uses the default refund type. The method does not return any value.
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
/// Asynchronously tests if refunding an invalid amount will fail by making a payment, attempting to refund an amount greater than the payment, and asserting that the response error reason is as expected.
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
/// Asynchronously makes a payment and refund using the Stripe connector, then retries syncing the refund status until it matches the specified status. It then asserts that the refund status is successful.
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
/// Asynchronously triggers a manual sync for a captured refund with the Stripe API.
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
