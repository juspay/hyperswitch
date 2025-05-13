use std::str::FromStr;

use api_models::enums;
use hyperswitch_connectors::connectors::TabaPay;
use hyperswitch_domain_models::payment_method_data::{Card, PaymentMethodData};
use router::types::PaymentsAuthorizeRouterData;
use serial_test::serial;

use crate::connector_auth::ConnectorAuthentication;
use crate::utils::{self, ConnectorActions, PaymentInfo};

static CONNECTOR: TabaPay = TabaPay::new();
static CONNECTOR_NAME: &str = "tabapay";

fn get_card_payment_method_data() -> Option<PaymentMethodData> {
    Some(PaymentMethodData::Card(Card {
        card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
        card_exp_month: "02".to_string().into(),
        card_exp_year: "2024".to_string().into(),
        card_cvc: "123".to_string().into(),
        card_holder_name: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
        nick_name: None,
    }))
}

fn get_default_payment_info() -> Option<PaymentInfo> {
    Some(PaymentInfo {
        address: None,
        amount: 100,
        currency: enums::Currency::USD,
        ..Default::default()
    })
}

#[serial]
#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_card_payment_method_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    
    let capture_response = CONNECTOR
        .capture_payment(authorize_response.response, None)
        .await
        .expect("Capture payment response");
    
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

#[serial]
#[actix_web::test]
async fn should_make_payment_and_refund() {
    let response = CONNECTOR
        .make_payment(get_card_payment_method_data(), get_default_payment_info())
        .await
        .expect("Make payment response");
    
    assert_eq!(response.status, enums::AttemptStatus::Charged);
    
    let transaction_id = utils::get_connector_transaction_id(response.response);
    let refund_response = CONNECTOR
        .refund_payment(transaction_id, None, None)
        .await
        .expect("Refund payment response");
    
    assert_eq!(refund_response.status, enums::RefundStatus::Success);
}

#[serial]
#[actix_web::test]
async fn should_sync_refund() {
    let response = CONNECTOR
        .make_payment(get_card_payment_method_data(), get_default_payment_info())
        .await
        .expect("Make payment response");
        
    let transaction_id = utils::get_connector_transaction_id(response.response);
    let refund_response = CONNECTOR
        .refund_payment(transaction_id, None, None)
        .await
        .expect("Refund payment response");
        
    let refund_sync_response = CONNECTOR
        .sync_refund(refund_response.response, None, None)
        .await
        .expect("Sync refund response");
        
    assert_eq!(refund_sync_response.status, enums::RefundStatus::Success);
}