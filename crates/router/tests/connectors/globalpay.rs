use masking::Secret;
use router::types::{self, api, storage::enums};
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
                    country: Some(api_models::enums::CountryAlpha2::US),
                    ..Default::default()
                }),
                phone: None,
            }),
            ..Default::default()
        }),
        access_token: Some(types::AccessToken {
            token: "<access_token>".to_string(),
            expires: 18600,
        }),
        ..Default::default()
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
    let response = connector
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = Globalpay {};
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
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
            get_default_payment_info(),
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
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
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
    let response = connector
        .make_payment_and_refund(None, None, get_default_payment_info())
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
    let response = connector
        .authorize_and_void_payment(None, None, get_default_payment_info())
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = Globalpay {};
    let refund_response = connector
        .make_payment_and_refund(None, None, get_default_payment_info())
        .await
        .unwrap();
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
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
