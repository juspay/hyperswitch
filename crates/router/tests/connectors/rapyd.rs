use std::str::FromStr;

use futures::future::OptionFuture;
use masking::Secret;
use router::types::{self, api, storage::enums};
use serial_test::serial;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct Rapyd;
impl ConnectorActions for Rapyd {}
impl utils::Connector for Rapyd {
        /// Retrieves the connector data for the Rapyd connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Rapyd;
        types::api::ConnectorData {
            connector: Box::new(&Rapyd),
            connector_name: types::Connector::Rapyd,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieve the authentication token for the connector.
    ///
    /// This method retrieves the authentication token for the connector by creating a new `ConnectorAuthentication` instance, accessing the `rapyd` property, and converting it to a `ConnectorAuthType` using the `utils::to_connector_auth_type` function. If the `rapyd` property is missing, an error message is thrown.
    ///
    /// # Returns
    ///
    /// The authentication token in the form of a `ConnectorAuthType`.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .rapyd
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "rapyd" as a String.
    fn get_name(&self) -> String {
            "rapyd".to_string()
        }
}

#[actix_web::test]
/// Asynchronously authorizes a payment using Rapyd API with specific payment details
async fn should_only_authorize_payment() {
    let response = Rapyd {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2024".to_string()),
                    card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
                    card_cvc: Secret::new("123".to_string()),
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    card_issuing_country: None,
                    bank_code: None,
                    nick_name: Some(masking::Secret::new("nick_name".into())),
                }),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
/// Asynchronously authorizes and captures a payment using Rapyd API with the provided payment data.
/// 
async fn should_authorize_and_capture_payment() {
    let response = Rapyd {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2024".to_string()),
                    card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
                    card_cvc: Secret::new("123".to_string()),
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    card_issuing_country: None,
                    bank_code: None,
                    nick_name: Some(masking::Secret::new("nick_name".into())),
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously checks if the payment is already authorized and captures it if it is.
async fn should_capture_already_authorized_payment() {
    let connector = Rapyd {};
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .capture_payment(transaction_id, None, None)
                .await
                .unwrap()
                .status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Charged));
}

#[actix_web::test]
#[serial]
/// Asynchronously tests that attempting to void an already authorized payment fails. It does this by first authorizing a payment using the Rapyd connector, then attempting to void the payment using the same connector. The method asserts that the authorization response status is 'Authorized', gets the transaction ID from the response, and then uses it to attempt to void the payment. Finally, it asserts that the response from the voiding attempt is 'Failure', as Rapyd does not allow authorized payments to be voided.
async fn voiding_already_authorized_payment_fails() {
    let connector = Rapyd {};
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .void_payment(transaction_id, None, None)
                .await
                .unwrap()
                .status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Failure)); //rapyd doesn't allow authorize transaction to be voided
}

#[actix_web::test]
/// Asynchronously makes a successful payment using the Rapyd connector and then attempts to refund the payment. If the payment is successful, the method will check the refund status and assert that the refund was successful.
async fn should_refund_succeeded_payment() {
    let connector = Rapyd {};
    //make a successful payment
    let response = connector.make_payment(None, None).await.unwrap();

    //try refund for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(response.response) {
        let response = connector
            .refund_payment(transaction_id, None, None)
            .await
            .unwrap();
        assert_eq!(
            response.response.unwrap().refund_status,
            enums::RefundStatus::Success,
        );
    }
}

#[actix_web::test]
/// Asynchronously attempts to make a payment with an incorrect card number and asserts that the payment fails.
async fn should_fail_payment_for_incorrect_card_number() {
    let response = Rapyd {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("0000000000000000").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();

    assert!(response.response.is_err(), "The Payment pass");
}
