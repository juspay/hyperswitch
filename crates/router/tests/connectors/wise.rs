#[cfg(feature = "payouts")]
use api_models::payments::{Address, AddressDetails};
#[cfg(feature = "payouts")]
use masking::Secret;
use router::types;
#[cfg(feature = "payouts")]
use router::types::{api, storage::enums, PaymentAddress};

#[cfg(feature = "payouts")]
use crate::utils::PaymentInfo;
use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct WiseTest;
impl ConnectorActions for WiseTest {}
impl utils::Connector for WiseTest {
        /// This method returns the connector data for the Adyen connector. It creates a new instance of ConnectorData with Adyen as the connector, Adyen as the connector name, GetToken set to Connector, and merchant_connector_id set to None.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Adyen;
        types::api::ConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::Connector::Adyen,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    #[cfg(feature = "payouts")]
        /// This method returns the payout data for a Wise connector, including the connector itself, the connector name, and the method to retrieve the token.
    fn get_payout_data(&self) -> Option<types::api::PayoutConnectorData> {
        use router::connector::Wise;
        Some(types::api::PayoutConnectorData {
            connector: Box::new(&Wise),
            connector_name: types::PayoutConnectors::Wise,
            get_token: types::api::GetToken::Connector,
        })
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .wise
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "wise" as a String.
    fn get_name(&self) -> String {
        "wise".to_string()
    }
}

impl WiseTest {
    #[cfg(feature = "payouts")]
        /// Retrieves the payment information for the user,
    /// including country, currency, address, and payout method data.
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
                    bank_name: Some("Deutsche Bank".to_string()),
                    bank_country_code: Some(enums::CountryAlpha2::NL),
                    bank_city: Some("Amsterdam".to_string()),
                },
            ))),
            ..Default::default()
        })
    }
}

#[cfg(feature = "payouts")]
static CONNECTOR: WiseTest = WiseTest {};

/******************** Payouts test cases ********************/
// Creates a recipient at connector's end
#[cfg(feature = "payouts")]
#[actix_web::test]
/// Asynchronously creates a payout recipient using the bank payout type and payment information retrieved from WiseTest. 
/// Expects a response from the create_payout_recipient method and asserts that the status of the response is RequiresCreation.
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
/// Asynchronously creates a BACS payout by first creating a recipient and then creating the payout itself.
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
/// Asynchronously creates a payout recipient of type Bank and fulfills the payout, 
/// checking that the recipient is created and the payout is successfully fulfilled.
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
