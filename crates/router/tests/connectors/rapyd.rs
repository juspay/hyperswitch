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
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Rapyd;
        types::api::ConnectorData {
            connector: Box::new(&Rapyd),
            connector_name: types::Connector::Rapyd,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .rapyd
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "rapyd".to_string()
    }
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Rapyd {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4111111111111111".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2024".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                }),
                capture_method: Some(storage_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let response = Rapyd {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4111111111111111".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2024".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("123".to_string()),
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
async fn should_capture_already_authorized_payment() {
    let connector = Rapyd {};
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response);
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
async fn voiding_already_authorized_payment_fails() {
    let connector = Rapyd {};
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response);
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
async fn should_refund_succeeded_payment() {
    let connector = Rapyd {};
    //make a successful payment
    let response = connector.make_payment(None, None).await.unwrap();

    //try refund for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(response) {
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
async fn should_fail_payment_for_incorrect_card_number() {
    let response = Rapyd {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("0000000000000000".to_string()),
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
