use std::str::FromStr;

use actix_web::http::header::HeaderMap;
use api_models::{
    payments::PaymentIdType,
    webhooks::{ConnectorWebhookSecrets, IncomingWebhookEvent, ObjectReferenceId},
};
use hyperswitch_domain_models::{
    address::{Address, AddressDetails, PhoneDetails},
    payment_method_data::{BankTransferData, PaymentMethodData},
};
use hyperswitch_interfaces::webhooks::{IncomingWebhook, IncomingWebhookRequestDetails};
use hyperswitch_masking::Secret;
use router::{
    connector::Tdaypay,
    types::{self, api, storage::enums, Email},
};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct TdaypayTest;
impl ConnectorActions for TdaypayTest {}
impl utils::Connector for TdaypayTest {
    fn get_data(&self) -> api::ConnectorData {
        utils::construct_connector_data_old(
            Box::new(Tdaypay::new()),
            types::Connector::Tdaypay,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .tdaypay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "tdaypay".to_string()
    }
}

static CONNECTOR: TdaypayTest = TdaypayTest {};

fn billing_contact() -> utils::PaymentInfo {
    utils::PaymentInfo {
        address: Some(types::PaymentAddress::new(
            None,
            None,
            Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("John".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                    ..Default::default()
                }),
                phone: Some(PhoneDetails {
                    number: Some(Secret::new("11999999999".to_string())),
                    country_code: Some("+55".to_string()),
                }),
                email: Some(Email::from_str("customer@test.com").unwrap()),
            }),
            None,
        )),
        ..Default::default()
    }
}

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(billing_contact())
}

fn authorize_data(
    payment_method_type: enums::PaymentMethodType,
    payment_method_data: PaymentMethodData,
    currency: enums::Currency,
) -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        amount: 100,
        minor_amount: common_utils::types::MinorUnit::new(100),
        currency,
        payment_method_data,
        payment_method_type: Some(payment_method_type),
        confirm: true,
        router_return_url: Some(String::from("https://example.com/tdaypay/return")),
        webhook_url: Some(String::from("https://example.com/tdaypay/webhook")),
        enrolled_for_3ds: false,
        ..utils::PaymentAuthorizeType::default().0
    })
}

fn pix_payment_details() -> Option<types::PaymentsAuthorizeData> {
    authorize_data(
        enums::PaymentMethodType::Pix,
        PaymentMethodData::BankTransfer(Box::new(BankTransferData::Pix {
            pix_key: None,
            cpf: None,
            cnpj: None,
            source_bank_account_id: None,
            destination_bank_account_id: None,
            expiry_date: None,
        })),
        enums::Currency::BRL,
    )
}

fn local_bank_transfer_payment_details() -> Option<types::PaymentsAuthorizeData> {
    authorize_data(
        enums::PaymentMethodType::LocalBankTransfer,
        PaymentMethodData::BankTransfer(Box::new(BankTransferData::LocalBankTransfer {
            bank_code: Some("SPEI".to_string()),
        })),
        enums::Currency::MXN,
    )
}

fn pse_payment_details() -> Option<types::PaymentsAuthorizeData> {
    authorize_data(
        enums::PaymentMethodType::Pse,
        PaymentMethodData::BankTransfer(Box::new(BankTransferData::Pse {})),
        enums::Currency::COP,
    )
}

fn webhook_details<'a>(
    body: &'a [u8],
    headers: &'a HeaderMap,
) -> IncomingWebhookRequestDetails<'a> {
    IncomingWebhookRequestDetails {
        method: http::Method::POST,
        uri: "http://localhost/webhooks/tdaypay".parse().unwrap(),
        headers,
        body,
        query_params: String::new(),
    }
}

// Bank transfer authorize (automatic capture). TDayPay does not support cards,
// manual capture, refunds, or 3DS.
#[actix_web::test]
async fn should_authorize_pix_payment() {
    let response = CONNECTOR
        .make_payment(pix_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize PIX payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

#[actix_web::test]
async fn should_authorize_local_bank_transfer_payment() {
    let response = CONNECTOR
        .make_payment(
            local_bank_transfer_payment_details(),
            get_default_payment_info(),
        )
        .await
        .expect("Authorize local bank transfer payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

#[actix_web::test]
async fn should_authorize_pse_payment() {
    let response = CONNECTOR
        .make_payment(pse_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize PSE payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

#[actix_web::test]
async fn should_sync_pix_payment() {
    let authorize_response = CONNECTOR
        .make_payment(pix_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize PIX payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::AuthenticationPending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

#[actix_web::test]
async fn should_map_success_webhook_event() {
    let connector = Tdaypay::new();
    let body = br#"{"orderId":"ord_pix_1","mchOrderId":"pay_pix_1","orderStatus":"SUCCESS"}"#;
    let headers = HeaderMap::new();
    let event = connector
        .get_webhook_event_type(&webhook_details(body, &headers), None)
        .expect("webhook event type");
    assert_eq!(event, IncomingWebhookEvent::PaymentIntentSuccess);
}

#[actix_web::test]
async fn should_map_paying_webhook_event() {
    let connector = Tdaypay::new();
    let body = br#"{"orderId":"ord_bt_1","mchOrderId":"pay_bt_1","orderStatus":"PAYING"}"#;
    let headers = HeaderMap::new();
    let event = connector
        .get_webhook_event_type(&webhook_details(body, &headers), None)
        .expect("webhook event type");
    assert_eq!(event, IncomingWebhookEvent::PaymentIntentProcessing);
}

#[actix_web::test]
async fn should_map_failed_webhook_event() {
    let connector = Tdaypay::new();
    let body = br#"{"orderId":"ord_pse_1","mchOrderId":"pay_pse_1","orderStatus":"FAILED"}"#;
    let headers = HeaderMap::new();
    let event = connector
        .get_webhook_event_type(&webhook_details(body, &headers), None)
        .expect("webhook event type");
    assert_eq!(event, IncomingWebhookEvent::PaymentIntentFailure);
}

#[actix_web::test]
async fn should_read_webhook_payment_reference() {
    let connector = Tdaypay::new();
    let body = br#"{"orderId":"ord_pix_1","mchOrderId":"pay_pix_1","orderStatus":"SUCCESS"}"#;
    let headers = HeaderMap::new();
    let reference = connector
        .get_webhook_object_reference_id(&webhook_details(body, &headers))
        .expect("webhook object reference");
    match reference {
        ObjectReferenceId::PaymentId(PaymentIdType::ConnectorTransactionId(id)) => {
            assert_eq!(id, "ord_pix_1");
        }
        other => panic!("unexpected webhook reference: {other:?}"),
    }
}

#[actix_web::test]
async fn should_build_webhook_source_verification_message() {
    let connector = Tdaypay::new();
    let body = br#"{"orderId":"ord_pix_1","orderStatus":"SUCCESS"}"#;
    let headers = HeaderMap::new();
    let secrets = ConnectorWebhookSecrets {
        secret: b"merchant-key".to_vec(),
        additional_secret: None,
    };
    let message = connector
        .get_webhook_source_verification_message(
            &webhook_details(body, &headers),
            &common_utils::id_type::MerchantId::default(),
            &secrets,
        )
        .expect("webhook verification message");
    let mut expected = body.to_vec();
    expected.extend_from_slice(b"merchant-key");
    assert_eq!(message, expected);
}
