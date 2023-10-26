use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken, PaymentAddress};

use crate::{
    connector_auth,
    utils::{
        self, get_connector_transaction_id, Connector, ConnectorActions, PaymentAuthorizeType,
    },
};

#[derive(Clone, Copy)]
struct IatapayTest;
impl ConnectorActions for IatapayTest {}
impl Connector for IatapayTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Iatapay;
        types::api::ConnectorData {
            connector: Box::new(&Iatapay),
            connector_name: types::Connector::Iatapay,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .iatapay
                .expect("Missing connector authentication configuration")
                .into(),
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
        } => Some(AccessToken {
            token: api_key,
            expires: 60 * 5,
        }),
        _ => None,
    }
}

static CONNECTOR: IatapayTest = IatapayTest {};

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
                    country: Some(api_models::enums::CountryAlpha2::NL),
                    ..Default::default()
                }),
                phone: Some(api::PhoneDetails {
                    number: Some(Secret::new("1234567890".to_string())),
                    country_code: Some("+91".to_string()),
                }),
            }),
            ..Default::default()
        }),
        access_token: get_access_token(),
        return_url: Some(String::from("https://hyperswitch.io")),
        ..Default::default()
    })
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        router_return_url: Some("https://hyperswitch.io".to_string()),
        webhook_url: Some("https://hyperswitch.io".to_string()),
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
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
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
                webhook_url: Some("https://hyperswitch.io".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap_err().code,
        "BAD_REQUEST".to_string(),
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .refund_payment(
            "PWGKCZ91M4JJ0".to_string(),
            Some(types::RefundsData {
                refund_amount: 150,
                webhook_url: Some("https://hyperswitch.io".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The amount to be refunded (100) is greater than the unrefunded amount (10.00): the amount of the payment is 10.00 and the refunded amount is 0.00",
    );
}

#[actix_web::test]
async fn should_sync_payment() {
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "PE9OTYNP639XW".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

#[actix_web::test]
async fn should_sync_refund() {
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "R5DNXUW4EY6PQ".to_string(),
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
