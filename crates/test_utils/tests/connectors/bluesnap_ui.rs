use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct BluesnapSeleniumTest;

impl SeleniumTest for BluesnapSeleniumTest {
        /// Returns the name of the connector, which in this case is "bluesnap".
    fn get_connector_name(&self) -> String {
        "bluesnap".to_string()
    }
}

/// Asynchronously makes a 3DS payment using the provided WebDriver. This method triggers a series of events to simulate the 3DS payment process, including redirection, clicking on elements, entering a code, and asserting the presence of certain elements. If the payment is successfully made, it returns Ok(()), otherwise it returns a WebDriverError.

async fn should_make_3ds_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = BluesnapSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/200"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::RunIf(
                Assert::IsElePresent(By::Id("Cardinal-CCA-IFrame")),
                vec![
                    Event::Trigger(Trigger::SwitchFrame(By::Id("Cardinal-CCA-IFrame"))),
                    Event::Assert(Assert::IsPresent("Enter your code below")),
                    Event::Trigger(Trigger::SendKeys(By::Name("challengeDataEntry"), "1234")),
                    Event::Trigger(Trigger::Click(By::ClassName("button.primary"))),
                ],
            ),
            Event::Trigger(Trigger::Sleep(10)),
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

/// This method makes a Google Pay payment using the provided WebDriver. It retrieves the public key from the automation configurations, constructs the payment URL using the key and other parameters, and then makes the payment using the provided WebDriver and the constructed URL. If the payment is successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_gpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = BluesnapSeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .bluesnap_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=bluesnap&gatewaymerchantid={pub_key}&amount=11.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a test for the should_make_3ds_payment function. It uses the tester! macro to run the test and confirm that the should_make_3ds_payment function is working as expected.
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_gpay_payment function. It utilizes the tester! macro to run the test case for making a payment using Google Pay.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
