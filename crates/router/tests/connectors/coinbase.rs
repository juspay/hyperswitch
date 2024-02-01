use api_models::payments::CryptoData;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct CoinbaseTest;
impl ConnectorActions for CoinbaseTest {}
impl utils::Connector for CoinbaseTest {
        /// This method returns the data required for connecting to the Coinbase API.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Coinbase;
        types::api::ConnectorData {
            connector: Box::new(&Coinbase),
            connector_name: types::Connector::Coinbase,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It creates a new instance of ConnectorAuthentication and expects the Coinbase authentication configuration to be present. It then converts the authentication type to the appropriate ConnectorAuthType and returns it.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .coinbase
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "coinbase".
    fn get_name(&self) -> String {
        "coinbase".to_string()
    }
}

static CONNECTOR: CoinbaseTest = CoinbaseTest {};

/// Retrieves the default payment information, if available.
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
        connector_meta_data: Some(json!({"pricing_type": "fixed_price"})),
        ..Default::default()
    })
}

/// Returns the payment method details for authorizing a payment, wrapped in an Option. If the payment method details are available, it returns Some with the payment method data, otherwise it returns None.
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
        customer_name: None,
        payment_experience: None,
        payment_method_type: None,
        session_token: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        router_return_url: Some(String::from("https://google.com/")),
        webhook_url: None,
        complete_authorize_url: None,
        capture_method: None,
        customer_id: None,
        surcharge_details: None,
        request_incremental_authorization: false,
        metadata: None,
    })
}

// Creates a payment using the manual capture flow
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR, and asserts that the response status is AuthenticationPending. It also checks if the response contains redirection data and asserts that it is not None.
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
/// Asynchronously checks if an authorized payment should be synced. It uses the PSync functionality to retry until the status matches the authorized status, and then checks if the response status is Charged. 
async fn should_sync_authorized_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "ADFY3789".to_string(),
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
/// Asynchronously checks if an unresolved payment should be synced. It retries syncing the payment data until the status matches 'Authorized', then performs a PSync request with the provided payment data and awaits the response. If the response status is 'Unresolved', the method passes; otherwise, it will fail with an assertion error.
async fn should_sync_unresolved_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "YJ6RFZXZ".to_string(),
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
/// Asynchronously checks if an expired payment should be synced. It sends a request to the connector to retry syncing a payment until the status matches 'Authorized'. It then expects a response and asserts that the status is 'Failure'.
async fn should_sync_expired_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "FZ89KDDB".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Failure);
}

// Synchronizes a cancelled transaction.
#[actix_web::test]
/// Asynchronously attempts to sync a cancelled payment by retrying until the payment status matches the authorized status. 
///
async fn should_sync_cancelled_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "C35AAXKF".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

