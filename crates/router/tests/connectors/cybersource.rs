use futures::future::OptionFuture;
use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentAuthorizeType},
};

struct Cybersource;
impl ConnectorActions for Cybersource {}
impl utils::Connector for Cybersource {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Cybersource;
        types::api::ConnectorData {
            connector: Box::new(&Cybersource),
            connector_name: types::Connector::Cybersource,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .cybersource
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "cybersource".to_string()
    }
}

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
        email: Some(Secret::new("abc@gmail.com".to_string())),
        ..PaymentAuthorizeType::default().0
    })
}
#[actix_web::test]
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
async fn should_authorize_and_capture_payment() {
    let connector = Cybersource {};
    let response = connector
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await;
    let sync_response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    utils::get_connector_transaction_id(response.unwrap()).unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    //cybersource takes sometime to settle the transaction,so it will be in pending for long time
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

#[actix_web::test]
async fn should_sync_capture_payment() {
    let sync_response = Cybersource {}
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "6736046645576085004953".to_string(),
                ),
                encoded_data: None,
                capture_method: None,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Cybersource {};
    let authorize_response = connector
        .authorize_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .capture_payment(transaction_id, None, get_default_payment_info())
                .await
                .unwrap()
                .status
        })
        .into();
    //cybersource takes sometime to settle the transaction,so it will be in pending for long time
    assert_eq!(response.await, Some(enums::AttemptStatus::Pending));
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Cybersource {};
    let authorize_response = connector
        .authorize_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .void_payment(transaction_id, None, get_default_payment_info())
                .await
                .unwrap()
                .status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Voided));
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Cybersource {};
    //make a successful payment
    let response = connector
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response).unwrap();
    let response = connector
        .refund_payment(transaction_id, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending, //cybersource takes sometime to refund the transaction,so it will be in pending state for long time
    );
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = Cybersource {};
    let response = connector
        .sync_refund(
            "6738063831816571404953".to_string(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending, //cybersource takes sometime to refund the transaction,so it will be in pending state for long time
    );
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = Cybersource {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("424242442424242".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_default_payment_authorize_data().unwrap()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure);
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Decline - Invalid account number".to_string(),);
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_exp_month() {
    let response = Cybersource {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4242424242424242".to_string()),
                    card_exp_month: Secret::new("101".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_default_payment_authorize_data().unwrap()
            }),
            get_default_payment_info(),
        )
        .await;
    let x = response.unwrap().response.unwrap_err();
    assert_eq!(
        x.message,
        r#"[{"field":"paymentInformation.card.expirationMonth","reason":"INVALID_DATA"}]"#
            .to_string(),
    );
}
