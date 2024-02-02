use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct MollieSeleniumTest;

impl SeleniumTest for MollieSeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "mollie".to_string()
    }
}

/// This method initiates a Mollie payment through PayPal using the provided web driver. It performs a series of events and assertions to simulate the payment process, including redirection to the checkout page, clicking on the submit button, asserting the presence of certain elements, and verifying the payment status. If the process is successful, it returns Ok(()); otherwise, it returns a WebDriverError.
async fn should_make_mollie_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/32"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// This method initiates a Mollie Sofort payment using the provided web driver. It creates a connection to MollieSeleniumTest, and then makes a series of web driver interactions to simulate the payment process, including redirection, clicking buttons, and asserting the presence of certain elements. If the payment process is successful, it returns Ok(()); otherwise, it returns a WebDriverError.
async fn should_make_mollie_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/29"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// Makes a Mollie ideal payment using the provided web driver by performing a series of events such as redirection, clicking on elements, and asserting the presence of certain elements. Returns a Result indicating success or a WebDriverError if an error occurs.
async fn should_make_mollie_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn: MollieSeleniumTest = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/36"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::ClassName(
                "payment-method-list--bordered",
            ))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Mollie EPS payment using the provided web driver. 
/// 
/// # Arguments
/// 
/// * `web_driver` - A WebDriver instance to use for interacting with the web page
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A Result indicating success or failure, with an associated WebDriverError in case of failure
/// 
async fn should_make_mollie_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/38"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// Makes a Mollie Giropay payment using the provided WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for making the payment.
///
async fn should_make_mollie_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/41"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// Asynchronously makes a payment using Mollie Bancontact card through the given WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for making the payment.
///
/// # Returns
///
/// * If successful, returns `Ok(())`.
/// * If an error occurs during the WebDriver interaction, returns a `WebDriverError`.
///
async fn should_make_mollie_bancontact_card_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/86"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// This method uses a WebDriver to make a payment through Mollie Przelewy24. It performs a series of events such as redirection, clicking on elements, and asserting the presence of certain elements, as well as the status of the payment. If successful, it returns a Result containing an empty value, otherwise it returns a WebDriverError.
async fn should_make_mollie_przelewy24_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/87"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

/// Asynchronously makes a 3DS payment using the Mollie payment gateway through a web driver.
async fn should_make_mollie_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/148"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This function is a test for the should_make_mollie_paypal_payment method. It uses the tester! macro to test the functionality of the should_make_mollie_paypal_payment method.
fn should_make_mollie_paypal_payment_test() {
    tester!(should_make_mollie_paypal_payment);
}

#[test]
#[serial]
/// This method is a test for the functionality to make a Mollie Sofort payment. It uses the tester macro to run the test case should_make_mollie_sofort_payment.
fn should_make_mollie_sofort_payment_test() {
    tester!(should_make_mollie_sofort_payment);
}

#[test]
#[serial]
/// Executes a test to ensure that the method should_make_mollie_ideal_payment correctly makes a payment using the Mollie iDeal payment method.
fn should_make_mollie_ideal_payment_test() {
    tester!(should_make_mollie_ideal_payment);
}

#[test]
#[serial]
/// This method is a test function that checks if the should_make_mollie_eps_payment method works as expected.
fn should_make_mollie_eps_payment_test() {
    tester!(should_make_mollie_eps_payment);
}

#[test]
#[serial]
/// This method is a test for making a Mollie Giropay payment. It uses the tester! macro to run the should_make_mollie_giropay_payment method.
fn should_make_mollie_giropay_payment_test() {
    tester!(should_make_mollie_giropay_payment);
}

#[test]
#[serial]
/// This method is a test case for making a Mollie Bancontact card payment. It uses the `tester!` macro to run the `should_make_mollie_bancontact_card_payment` function.
fn should_make_mollie_bancontact_card_payment_test() {
    tester!(should_make_mollie_bancontact_card_payment);
}

#[test]
#[serial]
/// This method is a test function for making a payment using the Mollie Przelewy24 payment method. It uses the `tester!` macro to run the test case for the `should_make_mollie_przelewy24_payment` method.
fn should_make_mollie_przelewy24_payment_test() {
    tester!(should_make_mollie_przelewy24_payment);
}

#[test]
#[serial]
/// Runs a test to check if the should_make_mollie_3ds_payment method works correctly.
fn should_make_mollie_3ds_payment_test() {
    tester!(should_make_mollie_3ds_payment);
}
