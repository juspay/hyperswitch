use common_utils::types::MinorUnit;
use hyperswitch_domain_models::payment_method_data::{EsewaData, PaymentMethodData, WalletData};
use router::{
    core::errors::ConnectorError,
    types::{self, api, storage::enums},
};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

// These tests check what the connector rejects before any HTTP call: the test harness's
// MockApiClient does not send requests. Calls to eSewa are covered by unit tests on
// recorded UAT responses in `esewa/transformers.rs`.

#[derive(Clone, Copy)]
struct EsewaTest;
impl ConnectorActions for EsewaTest {}
impl utils::Connector for EsewaTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Esewa;
        utils::construct_connector_data_old(
            Box::new(Esewa::new()),
            types::Connector::Esewa,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .esewa
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "esewa".to_string()
    }
}

static CONNECTOR: EsewaTest = EsewaTest {};

fn esewa_payment_data(currency: enums::Currency) -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::Wallet(WalletData::Esewa(Box::new(EsewaData {}))),
        amount: 10000,
        minor_amount: MinorUnit::new(10000),
        currency,
        router_return_url: Some(
            "http://localhost:8080/payments/pay_test/merchant_test/redirect/response/esewa"
                .to_string(),
        ),
        ..utils::PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_reject_currencies_other_than_npr() {
    let error = CONNECTOR
        .authorize_payment(esewa_payment_data(enums::Currency::USD), None)
        .await
        .expect_err("USD payment should be rejected");
    assert!(matches!(
        error.current_context(),
        ConnectorError::NotSupported { .. }
    ));
}

#[actix_web::test]
async fn should_reject_card_payments() {
    let error = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::NPR,
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect_err("Card payment should be rejected");
    assert!(matches!(
        error.current_context(),
        ConnectorError::NotImplemented(_)
    ));
}

#[actix_web::test]
async fn should_not_support_refunds() {
    let error = CONNECTOR
        .refund_payment("pay_test_1".to_string(), None, None)
        .await
        .expect_err("Refund should be rejected");
    assert!(matches!(
        error.current_context(),
        ConnectorError::NotSupported { .. }
    ));
}
