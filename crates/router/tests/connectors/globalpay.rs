use std::{thread::sleep, time::Duration};

use futures::future::OptionFuture;
use masking::Secret;
use router::types::{
    self,
    api::{self},
    storage::enums,
};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

struct Globalpay;
impl ConnectorActions for Globalpay {}
impl utils::Connector for Globalpay {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Globalpay;
        types::api::ConnectorData {
            connector: Box::new(&Globalpay),
            connector_name: types::Connector::Globalpay,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .globalpay
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "globalpay".to_string()
    }

    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        Some(json!({"account_name": "transaction_processing"}))
    }
}

fn get_default_payment_info() -> Option<PaymentInfo> {
    Some(PaymentInfo {
        address: Some(types::PaymentAddress {
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    country: Some("US".to_string()),
                    ..Default::default()
                }),
                phone: None,
            }),
            ..Default::default()
        }),
        auth_type: None,
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Globalpay {}
        .authorize_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let response = Globalpay {}
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Globalpay {};
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
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
    assert_eq!(response.await, Some(enums::AttemptStatus::Charged));
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = Globalpay {};
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    sleep(Duration::from_secs(5)); // to avoid 404 error as globalpay takes some time to process the new transaction
    let response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = Globalpay {}
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
    let x = response.status;
    assert_eq!(x, enums::AttemptStatus::Failure);
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Globalpay {};
    //make a successful payment
    let response = connector
        .make_payment(None, get_default_payment_info())
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
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Globalpay {};
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .unwrap();
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
    assert_eq!(response.await, Some(enums::AttemptStatus::Voided));
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = Globalpay {};
    let response = connector
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    let transaction_id = utils::get_connector_transaction_id(response).unwrap();
    connector
        .refund_payment(transaction_id.clone(), None, get_default_payment_info())
        .await
        .unwrap();
    sleep(Duration::from_secs(5)); // to avoid 404 error as globalpay takes some time to process the new transaction
    let response = connector
        .sync_refund(transaction_id, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
