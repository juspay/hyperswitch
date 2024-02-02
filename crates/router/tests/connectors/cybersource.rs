use std::str::FromStr;

use common_utils::pii::Email;
use masking::Secret;
use router::types::{
    self, api,
    storage::{self, enums},
};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentAuthorizeType},
};
struct Cybersource;
impl ConnectorActions for Cybersource {}
impl utils::Connector for Cybersource {
        /// Returns a ConnectorData object containing the necessary information for the Cybersource connector.
    fn get_data(&self) -> types::api::ConnectorData {
            use router::connector::Cybersource;
            types::api::ConnectorData {
                connector: Box::new(&Cybersource),
                connector_name: types::Connector::Cybersource,
                get_token: types::api::GetToken::Connector,
                merchant_connector_id: None,
            }
        }
        /// Retrieves the authentication token for the connector.
    /// 
    /// # Returns
    /// 
    /// The authentication token in the form of `types::ConnectorAuthType`.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .cybersource
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }
        /// Returns the name "cybersource".
    fn get_name(&self) -> String {
        "cybersource".to_string()
    }
}

/// Retrieves the default payment information for the user, if available.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(types::PaymentAddress {
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    first_name: Some(Secret::new("first".to_string())),
                    last_name: Some(Secret::new("last".to_string())),
                    line1: Some(Secret::new("line1".to_string())),
                    line2: Some(Secret::new("line2".to_string())),
                    city: Some("city".to_string()),
                    zip: Some(Secret::new("zip".to_string())),
                    country: Some(api_models::enums::CountryAlpha2::IN),
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
/// Returns the default payment authorize data, which includes the currency set to USD and an email set to "abc@gmail.com".
fn get_default_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        currency: storage::enums::Currency::USD,
        email: Some(Email::from_str("abc@gmail.com").unwrap()),
        ..PaymentAuthorizeType::default().0
    })
}
#[actix_web::test]
/// Asynchronously authorizes a payment using Cybersource API and asserts that the payment status is authorized.
async fn should_only_authorize_payment() {
    let response = Cybersource {}
        .authorize_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}
#[actix_web::test]
/// Asynchronously makes a payment using the Cybersource API with default payment authorization data and payment information. 
/// It awaits the response and asserts that the status of the payment attempt is pending.
async fn should_make_payment() {
    let response = Cybersource {}
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}
#[actix_web::test]
/// Asynchronously attempts to capture an already authorized payment using the CyberSource payment connector. 
/// It calls the `authorize_and_capture_payment` method of the CyberSource connector with default payment authorization data, no custom options, and default payment information, and awaits the response. 
/// It then asserts that the response status is Pending.
async fn should_capture_already_authorized_payment() {
    let connector = Cybersource {};
    let response = connector
        .authorize_and_capture_payment(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Pending);
}
#[actix_web::test]
/// Asynchronously attempts to partially capture a payment that has already been authorized. 
/// It uses the Cybersource connector to authorize and capture the payment, specifying the amount to capture. 
/// The method then waits for the response and asserts that the status is pending.
async fn should_partially_capture_already_authorized_payment() {
    let connector = Cybersource {};
    let response = connector
        .authorize_and_capture_payment(
            get_default_payment_authorize_data(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Pending);
}

#[actix_web::test]
#[ignore = "Status field is missing in the response, Communication is being done with cybersource team"]
/// Asynchronously checks if the payment should be synced. It uses the Cybersource connector to retry syncing the payment until its status matches the specified status (Charged). It then asserts that the response status is Charged.
async fn should_sync_payment() {
    let connector = Cybersource {};
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "6699597903496176903954".to_string(),
                ),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}
#[actix_web::test]
/// This method voids an already authorized payment by calling the `authorize_and_void_payment` method
/// on the `Cybersource` connector with the default payment authorization data, a cancellation reason
/// provided by the customer, and default payment information. It then asserts that the response status
/// is `Voided`.
async fn should_void_already_authorized_payment() {
    let connector = Cybersource {};
    let response = connector
        .authorize_and_void_payment(
            get_default_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
/// Asynchronously tests that a payment fails for an invalid expiration month. It makes a payment using Cybersource API with an invalid expiration month, then checks the response for the expected error message and reason.
async fn should_fail_payment_for_invalid_exp_month() {
    let response = Cybersource {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("13".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_default_payment_authorize_data().unwrap()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "Declined - One or more fields in the request contains invalid data",
    );
    assert_eq!(
        x.reason,
        Some("paymentInformation.card.expirationMonth : INVALID_DATA".to_string())
    );
}
#[actix_web::test]
/// Asynchronously makes a payment using Cybersource, with the intention of the payment failing due to an invalid expiration year on the card.
async fn should_fail_payment_for_invalid_exp_year() {
    let response = Cybersource {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2022".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_default_payment_authorize_data().unwrap()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Decline - Expired card. You might also receive this if the expiration date you provided does not match the date the issuing bank has on file.",);
}
#[actix_web::test]
/// Asynchronously tests if a payment fails for an invalid card CVC by making a payment request to Cybersource with a card CVC that is known to be invalid. 
async fn should_fail_payment_for_invalid_card_cvc() {
    let response = Cybersource {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("2131233213".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_default_payment_authorize_data().unwrap()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "Declined - One or more fields in the request contains invalid data",
    );
    assert_eq!(
        x.reason,
        Some("paymentInformation.card.securityCode : INVALID_DATA".to_string())
    );
}
// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs a void payment operation for a reversed payment in the Cybersource payment system.
async fn should_fail_void_payment_for_reversed_payment() {
    let connector = Cybersource {};
    // Authorize
    let authorize_response = connector
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    // Void
    let void_response = connector
        .void_payment("6736046645576085004953".to_string(), None, None)
        .await
        .unwrap();
    let res = void_response.response.unwrap_err();
    assert_eq!(
        res.message,
        "Decline - The authorization has already been reversed."
    );
}
#[actix_web::test]
/// Asynchronously attempts to capture a payment using the Cybersource connector, expecting the operation to fail due to invalid payment data. 
async fn should_fail_capture_for_invalid_payment() {
    let connector = Cybersource {};
    let response = connector
        .capture_payment("12345".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(
        err.message,
        "Declined - One or more fields in the request contains invalid data"
    );
    assert_eq!(err.code, "InvalidData".to_string());
}
#[actix_web::test]
/// Asynchronously makes a payment and attempts to refund it. It uses the Cybersource connector to make the payment and then immediately attempts to refund it without any additional information. After the refund attempt, it asserts that the refund status in the response is pending. 
async fn should_refund_succeeded_payment() {
    let connector = Cybersource {};
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
        enums::RefundStatus::Pending,
    );
}
#[actix_web::test]
#[ignore = "Connector Error, needs to be looked into and fixed"]
/// Asynchronously initiates a manual refund for a previously captured payment. This method uses the Cybersource connector to perform an authorization capture and refund operation with default payment authorization data and payment information. It then asserts that the refund status of the response is pending.
async fn should_refund_manually_captured_payment() {
    let connector = Cybersource {};
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
        enums::RefundStatus::Pending,
    );
}
#[actix_web::test]
/// Asynchronously makes a payment and partially refunds a succeeded payment using the Cybersource connector. 
/// 
/// The method first creates a Cybersource connector, then makes a payment and issues a partial refund for the payment. 
/// The refund amount is set to 50, and the refund status is expected to be pending. 
/// 
async fn should_partially_refund_succeeded_payment() {
    let connector = Cybersource {};
    let refund_response = connector
        .make_payment_and_refund(
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
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously performs a partial refund for a manually captured payment using the Cybersource connector.
async fn should_partially_refund_manually_captured_payment() {
    let connector = Cybersource {};
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
        enums::RefundStatus::Pending,
    );
}

#[actix_web::test]
/// Asynchronously tests if a refund for an invalid amount should fail. It creates a Cybersource connector, makes a payment and attempts to refund an amount of 15000. It then asserts that the refund status is Pending.
async fn should_fail_refund_for_invalid_amount() {
    let connector = Cybersource {};
    let response = connector
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            Some(types::RefundsData {
                refund_amount: 15000,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}
#[actix_web::test]
#[ignore = "Status field is missing in the response, Communication is being done with cybersource team"]
/// Asynchronously checks if a refund should be synced with the given payment information. 
async fn should_sync_refund() {
    let connector = Cybersource {};
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "6699597076726585603955".to_string(),
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
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously performs the manual synchronization of a captured refund. This method is used to manually trigger the synchronization process for a captured refund, allowing the system to update the corresponding records and ensure that the refund is properly processed. 
async fn should_sync_manually_captured_refund() {
    // method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Checks if a payment that was automatically captured should be refunded.
async fn should_refund_auto_captured_payment() {
    // implementation
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously checks if a succeeded payment should be refunded multiple times.
async fn should_refund_succeeded_payment_multiple_times() {
    // Method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// This method is an asynchronous function that should handle the case where the refund amount is higher than the payment amount.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    // method implementation here
}
