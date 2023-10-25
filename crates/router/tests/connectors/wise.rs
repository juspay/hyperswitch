use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

struct WiseTest;
impl ConnectorActions for WiseTest {}
impl utils::Connector for WiseTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Adyen;
        types::api::ConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::Connector::Adyen,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_payout_data(&self) -> Option<types::api::PayoutConnectorData> {
        use router::connector::Wise;
        Some(types::api::PayoutConnectorData {
            connector: Box::new(&Wise),
            connector_name: types::PayoutConnectors::Wise,
            get_token: types::api::GetToken::Connector,
        })
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .wise
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "wise".to_string()
    }
}

impl WiseTest {
    fn get_payout_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            country: Some(api_models::enums::CountryAlpha2::NL),
            currency: Some(enums::Currency::GBP),
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::GB),
                        city: Some("London".to_string()),
                        zip: Some(Secret::new("10025".to_string())),
                        line1: Some(Secret::new("50 Branson Ave".to_string())),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            payout_method_data: Some(api::PayoutMethodData::Bank(api::payouts::BankPayout::Bacs(
                api::BacsBankTransfer {
                    bank_sort_code: "231470".to_string().into(),
                    bank_account_number: "28821822".to_string().into(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                },
            ))),
            ..Default::default()
        })
    }
}

static CONNECTOR: WiseTest = WiseTest {};

/******************** Payouts test cases ********************/
// Creates a recipient at connector's end
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_payout_recipient() {
    let payout_type = enums::PayoutType::Bank;
    let payment_info = WiseTest::get_payout_info();
    let response = CONNECTOR
        .create_payout_recipient(payout_type, payment_info)
        .await
        .expect("Payout recipient creation response");
    assert_eq!(
        response.status.unwrap(),
        enums::PayoutStatus::RequiresCreation
    );
}

// Create BACS payout
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_bacs_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = WiseTest::get_payout_info();
    // Create recipient
    let recipient_res = CONNECTOR
        .create_payout_recipient(payout_type.to_owned(), payout_info.to_owned())
        .await
        .expect("Payout recipient response");
    assert_eq!(
        recipient_res.status.unwrap(),
        enums::PayoutStatus::RequiresCreation
    );

    // Create payout
    let create_res: types::PayoutsResponseData = CONNECTOR
        .create_payout(
            Some(recipient_res.connector_payout_id),
            payout_type,
            payout_info,
        )
        .await
        .expect("Payout bank creation response");
    assert_eq!(
        create_res.status.unwrap(),
        enums::PayoutStatus::RequiresFulfillment
    );
}

// Create and fulfill BACS payout
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_and_fulfill_bacs_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = WiseTest::get_payout_info();
    // Create recipient
    let recipient_res = CONNECTOR
        .create_payout_recipient(payout_type.to_owned(), payout_info.to_owned())
        .await
        .expect("Payout recipient response");
    assert_eq!(
        recipient_res.status.unwrap(),
        enums::PayoutStatus::RequiresCreation
    );
    let response = CONNECTOR
        .create_and_fulfill_payout(
            Some(recipient_res.connector_payout_id),
            payout_type,
            payout_info,
        )
        .await
        .expect("Payout bank creation and fulfill response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Success);
}
