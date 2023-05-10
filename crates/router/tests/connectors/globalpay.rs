use std::str::FromStr;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use router::types::{self, api, storage::enums, AccessToken, ErrorResponse};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions, PaymentInfo},
};

struct Globalpay;
impl ConnectorActions for Globalpay {}
impl Connector for Globalpay {
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

async fn get_default_payment_info() -> Option<PaymentInfo> {
    let access_token = ACCESS_TOKEN.get().await.to_owned().unwrap();
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
        access_token: Some(access_token),
        connector_meta_data: CONNECTOR.get_connector_meta(),
        ..Default::default()
    })
}

static CONNECTOR: Globalpay = Globalpay {};

lazy_static! {
    static ref ACCESS_TOKEN: AsyncOnce<Result<AccessToken, ErrorResponse>> =
        AsyncOnce::new(async {
            CONNECTOR
                .generate_access_token(None)
                .await
                .expect("Access token response")
                .response
        });
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR
        .authorize_payment(None, payment_info)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR.make_payment(None, payment_info).await.unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            payment_info,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_payment() {
    let payment_info = get_default_payment_info().await;
    let authorize_response = CONNECTOR
        .authorize_payment(None, payment_info.clone())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            payment_info,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            payment_info,
        )
        .await
        .unwrap();
    let x = response.status;
    assert_eq!(x, enums::AttemptStatus::Failure);
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR
        .make_payment_and_refund(None, None, payment_info)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let payment_info = get_default_payment_info().await;
    let response = CONNECTOR
        .authorize_and_void_payment(None, None, payment_info)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
#[ignore = "Refund not supported"]
async fn should_sync_refund() {
    let payment_info = get_default_payment_info().await;
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, payment_info.clone())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            payment_info,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
