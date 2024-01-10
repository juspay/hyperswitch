use std::marker::PhantomData;

use api_models::payments::CryptoData;
use common_utils::pii;
use masking::Secret;
use router::{
    connector::coinbase::transformers::{CoinbasePaymentsRequest, LocalPrice},
    core::errors::ConnectorError,
    types::{self, api, storage::enums, PaymentAddress},
};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct CoinbaseTest;
impl ConnectorActions for CoinbaseTest {}
impl utils::Connector for CoinbaseTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Coinbase;
        types::api::ConnectorData {
            connector: Box::new(&Coinbase),
            connector_name: types::Connector::Coinbase,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .coinbase
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "coinbase".to_string()
    }
}

static CONNECTOR: CoinbaseTest = CoinbaseTest {};

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

fn payment_method_details() -> types::PaymentsAuthorizeData {
    types::PaymentsAuthorizeData {
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
        webhook_url: None,
        complete_authorize_url: None,
        capture_method: None,
        customer_id: None,
        surcharge_details: None,
        request_incremental_authorization: false,
        metadata: None,
    }
}

fn construct_payment_router_data(
    connector_meta_data: Option<pii::SecretSerdeValue>,
) -> types::PaymentsAuthorizeRouterData {
    let connector_auth_type = types::ConnectorAuthType::HeaderKey {
        api_key: Secret::new("api_key".to_string()),
    };

    types::RouterData {
        flow: PhantomData,
        merchant_id: String::from("Coinbase"),
        customer_id: Some(String::from("Coinbase")),
        connector: "Coinbase".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        status: Default::default(),
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethod::Card,
        connector_auth_type,
        description: Some("This is a test".to_string()),
        return_url: None,
        request: payment_method_details(),
        response: Err(Default::default()),
        payment_method_id: None,
        address: Default::default(),
        connector_meta_data,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        apple_pay_flow: None,
        external_latency: None,
        frm_metadata: None,
    }
}

// Creates a payment using the manual capture flow
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(Some(payment_method_details()), get_default_payment_info())
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

#[test]
fn coinbase_payments_request_try_from_works() {
    // `connector_meta_data` as `None` - should fail
    assert_eq!(
        CoinbasePaymentsRequest::try_from(&construct_payment_router_data(None))
            .unwrap_err()
            .current_context(),
        &ConnectorError::InvalidConnectorConfig {
            config: "metadata"
        },
    );

    // `connector_meta_data` as empty json - should fail
    assert_eq!(
        CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
            serde_json::json!({})
        ))))
        .unwrap_err()
        .current_context(),
        &ConnectorError::InvalidConnectorConfig {
            config: "metadata"
        },
    );

    // `connector_meta_data` as json with missing `pricing_type`  - should fail
    assert_eq!(
        CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
            serde_json::json!({ "wrong_type" : "blah" })
        ))))
        .unwrap_err()
        .current_context(),
        &ConnectorError::InvalidConnectorConfig {
            config: "metadata"
        },
    );

    // `connector_meta_data` as json with correct `pricing_type`  - ok
    assert_eq!(
        CoinbasePaymentsRequest::try_from(&construct_payment_router_data(Some(Secret::new(
            serde_json::json!({ "pricing_type" : "fixed_price" })
        ))))
        .unwrap(),
        CoinbasePaymentsRequest {
            name: None,
            description: Some("This is a test".to_string()),
            pricing_type: "fixed_price".to_string(),
            local_price: LocalPrice {
                amount: "1".to_string(),
                currency: "USD".to_string()
            },
            redirect_url: "https://google.com/".to_string(),
            cancel_url: "https://google.com/".to_string(),
        }
    );
}
