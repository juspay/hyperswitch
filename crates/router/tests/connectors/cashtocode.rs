use api_models::payments::{Address, AddressDetails};
use router::types::{self, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct CashtocodeTest;
impl ConnectorActions for CashtocodeTest {}
impl utils::Connector for CashtocodeTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Cashtocode;
        types::api::ConnectorData {
            connector: Box::new(&Cashtocode),
            connector_name: types::Connector::Cashtocode,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .cashtocode
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "cashtocode".to_string()
    }
}

static CONNECTOR: CashtocodeTest = CashtocodeTest {};

impl CashtocodeTest {
    fn get_payment_authorize_data(
        payment_method_type: Option<enums::PaymentMethodType>,
        payment_method_data: types::api::PaymentMethodData,
    ) -> Option<types::PaymentsAuthorizeData> {
        Some(types::PaymentsAuthorizeData {
            amount: 3500,
            currency: enums::Currency::USD,
            payment_method_data,
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            payment_experience: None,
            payment_method_type,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            router_return_url: Some(String::from("http://localhost:8080")),
            webhook_url: None,
            complete_authorize_url: None,
            customer_id: Some("John Doe".to_owned()),
        })
    }

    fn get_payment_info() -> Option<utils::PaymentInfo> {
        Some(utils::PaymentInfo {
            address: Some(types::PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            return_url: Some("https://google.com".to_owned()),
            ..Default::default()
        })
    }
}

//fetch payurl for payment's create
#[actix_web::test]
async fn should_fetch_pay_url_classic() {
    let authorize_response = CONNECTOR
        .make_payment(
            CashtocodeTest::get_payment_authorize_data(
                Some(enums::PaymentMethodType::ClassicReward),
                api_models::payments::PaymentMethodData::Reward(api_models::payments::RewardData {
                    merchant_id: "1bc20b0a".to_owned(),
                }),
            ),
            CashtocodeTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        authorize_response.status,
        enums::AttemptStatus::AuthenticationPending
    );
}

#[actix_web::test]
async fn should_fetch_pay_url_evoucher() {
    let authorize_response = CONNECTOR
        .make_payment(
            CashtocodeTest::get_payment_authorize_data(
                Some(enums::PaymentMethodType::Evoucher),
                api_models::payments::PaymentMethodData::Reward(api_models::payments::RewardData {
                    merchant_id: "befb46ee".to_owned(),
                }),
            ),
            CashtocodeTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        authorize_response.status,
        enums::AttemptStatus::AuthenticationPending
    );
}
