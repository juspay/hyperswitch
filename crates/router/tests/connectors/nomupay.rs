use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions, PaymentInfo};

#[derive(Clone, Copy)]
struct NomupayTest;
impl ConnectorActions for NomupayTest {}

impl utils::Connector for NomupayTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Nomupay;
        utils::construct_connector_data_old(
            Box::new(Nomupay::new()),
            types::Connector::Plaid,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nomupay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "nomupay".to_string()
    }
}

impl NomupayTest {
    fn get_payout_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            currency: Some(enums::Currency::GBP),
            address: Some(PaymentAddress::new(
                None,
                Some(
                    Address {
                        address: Some(AddressDetails {
                            country: Some(api_models::enums::CountryAlpha2::GB),
                            city: Some("London".to_string()),
                            zip: Some(Secret::new("10025".to_string())),
                            line1: Some(Secret::new("50 Branson Ave".to_string())),
                            ..Default::default()
                        }),
                        phone: None,
                        email: None,
                    }
                    .into(),
                ),
                None,
                None,
            )),
            payout_method_data: Some(api::PayoutMethodData::Bank(api::payouts::BankPayout::Sepa(
                api::SepaBankTransfer {
                    bank_name: Some("Deutsche Bank".to_string()),
                    bank_country_code: Some(enums::CountryAlpha2::DE),
                    bank_city: Some("Munich".to_string()),
                    iban: Secret::new("DE57331060435647542639".to_string()),
                    bic: Some(Secret::new("DEUTDE5M551".to_string())),
                },
            ))),
            ..Default::default()
        })
    }
}

static CONNECTOR: NomupayTest = NomupayTest {};

/******************** Payouts test cases ********************/
// Creates a recipient at connector's end

#[actix_web::test]
async fn should_create_payout_recipient() {
    let payout_type = enums::PayoutType::Bank;
    let payment_info = NomupayTest::get_payout_info();
    let response = CONNECTOR
        .create_payout_recipient(payout_type, payment_info)
        .await
        .expect("Payout recipient creation response");
    assert_eq!(
        response.status.unwrap(),
        enums::PayoutStatus::RequiresCreation
    );
}

// Create SEPA payout

#[actix_web::test]
async fn should_create_bacs_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = NomupayTest::get_payout_info();
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
        .create_payout(recipient_res.connector_payout_id, payout_type, payout_info)
        .await
        .expect("Payout bank creation response");
    assert_eq!(
        create_res.status.unwrap(),
        enums::PayoutStatus::RequiresFulfillment
    );
}

// Create and fulfill SEPA payout

#[actix_web::test]
async fn should_create_and_fulfill_bacs_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = NomupayTest::get_payout_info();
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
        .create_and_fulfill_payout(recipient_res.connector_payout_id, payout_type, payout_info)
        .await
        .expect("Payout bank creation and fulfill response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Success);
}
