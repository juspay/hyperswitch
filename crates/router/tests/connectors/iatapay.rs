
use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken};

use crate::{
    connector_auth,
    utils::{
        self, get_connector_transaction_id, Connector, ConnectorActions, PaymentAuthorizeType,
    },
};

#[derive(Clone, Copy)]
struct IatapayTest;
impl ConnectorActions for IatapayTest {}
impl utils::Connector for IatapayTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Iatapay;
        types::api::ConnectorData {
            connector: Box::new(&Iatapay),
            connector_name: types::Connector::Iatapay,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .iatapay
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "iatapay".to_string()
    }
}

fn get_access_token() -> Option<AccessToken> {
    let connector = IatapayTest {};
    match connector.get_auth_token() {
        types::ConnectorAuthType::SignatureKey {
            api_key,
            key1: _,
            api_secret: _,
        } => {
            Some(AccessToken {
                token: api_key,
                expires: 60 * 5,
            })
        }
        _ => None,
    }
}

static CONNECTOR: IatapayTest = IatapayTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        currency: enums::Currency::EUR,
        ..PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests

// Creates a payment checking if its status is "requires_customer_action" for redirectinal flow
#[actix_web::test]
async fn should_only_create_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(
        response.status.to_string(),
        enums::IntentStatus::RequiresCustomerAction.to_string()
    );
}

//refund on an unsuccessed payments
#[actix_web::test]
async fn should_fail_for_refund_on_unsuccessed_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let refund_response = CONNECTOR
        .refund_payment(
            get_connector_transaction_id(response.response).unwrap(),
            Some(types::RefundsData {
                refund_amount: response.request.amount,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap_err().message,
        "The payment has not succeeded yet. Please pass a successful payment to initiate refund"
            .to_string(),
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                connector_refund_id: Some("PWGKCZ91M4JJ0".to_string()),
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Refund amount exceeds the payment amount",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
