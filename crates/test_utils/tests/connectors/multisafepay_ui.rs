use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct MultisafepaySeleniumTest;

impl SeleniumTest for MultisafepaySeleniumTest {
        /// Returns the connector name "multisafepay".
    fn get_connector_name(&self) -> String {
        "multisafepay".to_string()
    }
}

/// Asynchronously makes a 3DS payment using Multisafepay and checks if the payment was successful.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver used to interact with the web page.
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError.
///
async fn should_make_multisafepay_3ds_payment_success(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/207"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// This method attempts to make a Multisafepay 3DS payment and expects it to fail. It uses a WebDriver to perform the necessary actions to trigger the payment and then asserts that the "failed" element is present on the page, indicating a failed payment. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_multisafepay_3ds_payment_failed(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/93"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("failed")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Google Pay payment using the provided web driver. It initiates a redirection payment process using MultisafepaySeleniumTest connection and a series of events such as triggering a click, asserting presence of an element, etc.
async fn should_make_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/153"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[class='btn btn-default']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a PayPal payment using the provided web driver. 
/// 
/// # Arguments
/// 
/// * `web_driver` - The web driver to use for the payment process.
/// 
/// # Returns
/// 
/// A `Result` indicating success or an error of type `WebDriverError`.
/// 
async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/154"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='btn btn-msp-success btn-block']",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This function is a test case for making a multisafepay 3DS payment successfully. 
fn should_make_multisafepay_3ds_payment_success_test() {
    tester!(should_make_multisafepay_3ds_payment_success);
}

#[test]
#[serial]
/// This method is a test function that checks whether the should_make_multisafepay_3ds_payment_failed
/// function returns the expected result using the tester macro.
fn should_make_multisafepay_3ds_payment_failed_test() {
    tester!(should_make_multisafepay_3ds_payment_failed);
}

#[test]
#[serial]
#[ignore]
/// This method tests the "should_make_gpay_payment" function by using the tester macro to ensure that the GPay payment is made successfully.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
/// This method tests the functionality of making a PayPal payment.
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}
