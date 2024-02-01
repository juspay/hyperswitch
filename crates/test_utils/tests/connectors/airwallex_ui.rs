use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AirwallexSeleniumTest;

impl SeleniumTest for AirwallexSeleniumTest {
        /// Returns the name of the connector, which is "airwallex".
    fn get_connector_name(&self) -> String {
        "airwallex".to_string()
    }
}

/// This method initiates a 3D Secure payment using Airwallex by performing a series of events using the provided WebDriver. It navigates to the specified checkout URL, clicks on the card-submit button, enters the challenge data, and performs assertions to verify the payment process. If the payment is successful, it returns Ok(()), otherwise it returns an error of type WebDriverError.
async fn should_make_airwallex_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AirwallexSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/85"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(
                Selector::Title,
                "Airwallex - Create 3D Secure Payment",
            )),
            Event::Trigger(Trigger::SendKeys(By::Id("challengeDataEntry"), "1234")),
            Event::Trigger(Trigger::Click(By::Id("submit"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

/// This method makes a payment using Airwallex as the gateway through Google Pay. It first retrieves the merchant name from the configuration, then uses it to construct the payment URL. After that, it simulates a series of events and assertions to complete the payment process using the provided web driver. If successful, it returns a `Result` with an empty value, otherwise it returns a `WebDriverError`.
async fn should_make_airwallex_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AirwallexSeleniumTest {};
    let merchant_name = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .airwallex_merchant_name
        .unwrap();
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=airwallex&gatewaymerchantid={merchant_name}&amount=70.00&country=US&currency=USD"),
        vec![
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(Selector::Title, "Airwallex - Create 3D Secure Payment")),
            Event::Trigger(Trigger::SendKeys(By::Id("challengeDataEntry"), "1234")),
            Event::Trigger(Trigger::Click(By::Id("submit"))),
            Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

#[test]
#[serial]
/// The `should_make_airwallex_3ds_payment_test` function is used to test the functionality of making a 3D Secure payment using the Airwallex payment method. It utilizes the `tester!` macro to execute the `should_make_airwallex_3ds_payment` test case.
fn should_make_airwallex_3ds_payment_test() {
    tester!(should_make_airwallex_3ds_payment);
}

#[test]
#[serial]
#[ignore]
/// This method is a test for making a payment using Airwallex and Google Pay. It uses the tester macro to run the actual test case should_make_airwallex_gpay_payment.
fn should_make_airwallex_gpay_payment_test() {
    tester!(should_make_airwallex_gpay_payment);
}
