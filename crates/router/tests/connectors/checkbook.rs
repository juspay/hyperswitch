use std::str::FromStr;

use hyperswitch_domain_models::address::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, Email};

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct CheckbookTest;
impl ConnectorActions for CheckbookTest {}
impl utils::Connector for CheckbookTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Checkbook;
        utils::construct_connector_data_old(
            Box::new(Checkbook::new()),
            types::Connector::Checkbook,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::BodyKey {
            key1: Secret::new("dummy_publishable_key".to_string()),
            api_key: Secret::new("dummy_secret_key".to_string()),
        }
    }

    fn get_name(&self) -> String {
        "checkbook".to_string()
    }
}

static CONNECTOR: CheckbookTest = CheckbookTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(types::PaymentAddress::new(
            None,
            None,
            Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("John".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                    ..Default::default()
                }),
                phone: None,
                email: Some(Email::from_str("abc@gmail.com").unwrap()),
            }),
            None,
        )),
        ..Default::default()
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

// Creates a payment.
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

// Synchronizes a payment.
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
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

// Voids a payment.
#[actix_web::test]
async fn should_void_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}
