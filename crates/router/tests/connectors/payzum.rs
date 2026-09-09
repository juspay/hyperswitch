use common_utils::types::MinorUnit;
use router::types::{self, api, domain, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct PayzumTest;
impl ConnectorActions for PayzumTest {}
impl utils::Connector for PayzumTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Payzum;
        utils::construct_connector_data_old(
            Box::new(Payzum::new()),
            types::Connector::Payzum,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .payzum
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "payzum".to_string()
    }
}

static CONNECTOR: PayzumTest = PayzumTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        // Above the per-network minimums so a live run creates a real invoice.
        amount: 2999,
        minor_amount: MinorUnit::new(2999),
        currency: enums::Currency::USD,
        payment_method_data: domain::PaymentMethodData::Crypto(domain::CryptoData {
            pay_currency: None,
            network: None,
        }),
        confirm: true,
        router_return_url: Some(String::from("https://example.com/return")),
        webhook_url: Some(String::from("https://example.com/webhooks/payzum")),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Creating an invoice yields a redirect to the hosted checkout and the
// payment stays open until the buyer pays.
#[actix_web::test]
async fn should_create_invoice_and_redirect() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
    let resp = response.response.ok().unwrap();
    let endpoint = match resp {
        types::PaymentsResponseData::TransactionResponse {
            redirection_data, ..
        } => *redirection_data,
        _ => None,
    };
    assert!(endpoint.is_some())
}

// A freshly created, unpaid invoice syncs back as still awaiting the buyer.
#[actix_web::test]
async fn should_sync_open_invoice() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response)
        .expect("connector transaction id");
    let response = CONNECTOR
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(txn_id),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}
