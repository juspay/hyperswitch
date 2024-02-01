use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct NoonSeleniumTest;

impl SeleniumTest for NoonSeleniumTest {
        /// This method returns the name of the connector as a string.
    fn get_connector_name(&self) -> String {
        "noon".to_string()
    }
}

/// This method initiates a 3DS payment flow on the Noon website using the provided web driver. It constructs a series of events to simulate the user's interaction with the payment page, such as clicking buttons, entering input, and waiting for the payment to be processed. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_noon_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/176"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("redirectTo3ds1Frame"))),
            Event::Trigger(Trigger::SwitchFrame(By::Css("iframe[frameborder='0']"))),
            Event::Trigger(Trigger::SendKeys(By::Css("input.input-field"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("input.button.primary"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// This method makes a payment using 3DS mandate on the Noon website using the provided WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for making the payment.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A Result indicating success or an error of type WebDriverError.
///
async fn should_make_noon_3ds_mandate_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/214"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("redirectTo3ds1Frame"))),
            Event::Trigger(Trigger::SwitchFrame(By::Css("iframe[frameborder='0']"))),
            Event::Trigger(Trigger::SendKeys(By::Css("input.input-field"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("input.button.primary"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("a.btn"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}
/// This method makes a non-3DS mandate payment on Noon using the provided web driver. It performs a series of events to complete the payment process and asserts the presence of certain elements to ensure the payment is successful.
async fn should_make_noon_non_3ds_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/215"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("a.btn"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// A method to test the functionality of making a Noon 3DS payment.
fn should_make_noon_3ds_payment_test() {
    tester!(should_make_noon_3ds_payment);
}

#[test]
#[serial]
/// This method is a test for making a 3DS mandate payment using the `should_make_noon_3ds_mandate_payment` function from the `tester` macro.
fn should_make_noon_3ds_mandate_payment_test() {
    tester!(should_make_noon_3ds_mandate_payment);
}

#[test]
#[serial]
/// This method is a test for the `should_make_noon_non_3ds_mandate_payment` function to ensure that it correctly makes a non-3DS (3D-Secure) mandate payment at noon. 
fn should_make_noon_non_3ds_mandate_payment_test() {
    tester!(should_make_noon_non_3ds_mandate_payment);
}
