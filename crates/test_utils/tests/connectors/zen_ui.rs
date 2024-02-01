use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct ZenSeleniumTest;

impl SeleniumTest for ZenSeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "zen".to_string()
    }
}

/// Asynchronously makes a payment using the ZenSeleniumTest instance with the provided WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for making the payment
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A Result indicating success or an error of type WebDriverError
///
async fn should_make_zen_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let mycon = ZenSeleniumTest {};
    mycon
        .make_redirection_payment(
            web_driver,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/201"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(10)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
                Event::Trigger(Trigger::Sleep(3)),
                Event::Assert(Assert::IsPresent("succeeded")),
            ],
        )
        .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a test case for the should_make_zen_3ds_payment function, which is used to test the functionality of making a 3DS payment using the Zen payment system.
fn should_make_zen_3ds_payment_test() {
    tester!(should_make_zen_3ds_payment);
}
