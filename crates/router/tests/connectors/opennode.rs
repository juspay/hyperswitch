use api_models::payments::CryptoData;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct OpennodeTest;
impl ConnectorActions for OpennodeTest {}
impl utils::Connector for OpennodeTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Opennode;
        types::api::ConnectorData {
            connector: Box::new(&Opennode),
            connector_name: types::Connector::Opennode,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .opennode
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "opennode".to_string()
    }
}

static CONNECTOR: OpennodeTest = OpennodeTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(PaymentAddress {
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
        return_url: Some(String::from("https://google.com")),
        ..Default::default()
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        amount: 1,
        currency: enums::Currency::USD,
        payment_method_data: types::api::PaymentMethodData::Crypto(CryptoData {
            pay_currency: None,
        }),
        confirm: true,
        statement_descriptor_suffix: None,
        statement_descriptor: None,
        setup_future_usage: None,
        mandate_id: None,
        off_session: None,
        setup_mandate_details: None,
        // capture_method: Some(capture_method),
        browser_info: None,
        order_details: None,
        order_category: None,
        email: None,
        payment_experience: None,
        payment_method_type: None,
        session_token: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        router_return_url: Some(String::from("https://google.com/")),
        webhook_url: Some(String::from("https://google.com/")),
        complete_authorize_url: None,
        capture_method: None,
        customer_id: None,
        surcharge_details: None,
    })
}

// Creates a payment using the manual capture flow
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
    let resp = response.response.ok().unwrap();
    let endpoint = match resp {
        types::PaymentsResponseData::TransactionResponse {
            redirection_data, ..
        } => Some(redirection_data),
        _ => None,
    };
    assert!(endpoint.is_some())
}

// Synchronizes a successful transaction.
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "5adebfb1-802e-432b-8b42-5db4b754b2eb".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a unresolved(underpaid) transaction.
#[actix_web::test]
async fn should_sync_unresolved_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "4cf63e6b-5135-49cb-997f-6e0b30fecebc".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Unresolved);
}

// Synchronizes a expired transaction.
#[actix_web::test]
async fn should_sync_expired_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "c36a097a-5091-4317-8749-80343a71c1c4".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Failure);
}
