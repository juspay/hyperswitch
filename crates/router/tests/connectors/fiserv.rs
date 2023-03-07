use futures::future::OptionFuture;
use masking::Secret;
use router::types::{self, api, storage::enums};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct Fiserv;
impl ConnectorActions for Fiserv {}

impl utils::Connector for Fiserv {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Fiserv;
        types::api::ConnectorData {
            connector: Box::new(&Fiserv),
            connector_name: types::Connector::Fiserv,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .fiserv
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "fiserv".to_string()
    }

    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        Some(json!({"terminalId": "10000001"}))
    }
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = Fiserv {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new("4005550000000019".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2035".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    card_issuer: None,
                    card_network: None,
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
    let response = Fiserv {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new("4005550000000019".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2035".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    card_issuer: None,
                    card_network: None,
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Fiserv {};
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new("4005550000000019".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2035".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    card_issuer: None,
                    card_network: None,
                }),
                capture_method: Some(storage_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
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
async fn should_fail_payment_for_missing_cvc() {
    let response = Fiserv {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new("4005550000000019".to_string()),
                    card_exp_month: Secret::new("02".to_string()),
                    card_exp_year: Secret::new("2035".to_string()),
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("".to_string()),
                    card_issuer: None,
                    card_network: None,
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Invalid or Missing Field Data".to_string(),);
}

#[ignore]
#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Fiserv {};
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
