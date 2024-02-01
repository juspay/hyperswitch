use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct PaypalSeleniumTest;

impl SeleniumTest for PaypalSeleniumTest {
        /// Returns the name of the connector, which is "paypal".
    fn get_connector_name(&self) -> String {
        "paypal".to_string()
    }
}

/// Asynchronously initiates a PayPal wallet payment using the provided web driver. It calls the `make_paypal_payment` method of `PaypalSeleniumTest` to simulate the payment process by performing a series of events and assertions. If the payment process is successful, it returns `Ok(())`, otherwise it returns a `WebDriverError`.
async fn should_make_paypal_paypal_wallet_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/21"),
        vec![
            Event::Trigger(Trigger::Click(By::Css("#payment-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a PayPal payment using the iDEAL method through the provided web driver. 
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver to use for making the payment.
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error with the WebDriver.
/// 
async fn should_make_paypal_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/181"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously performs a PayPal Giropay payment using the provided web driver. 
/// This method makes a series of WebDriver events to navigate to the PayPal checkout page, 
/// submit the payment, and assert the presence of the "processing" element. 
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver to use for performing the payment process.
/// 
/// # Returns
/// 
/// Returns a `Result` indicating success or an error of type `WebDriverError`.
/// 
async fn should_make_paypal_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/233"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Paypal Express (EPS) by performing a series of actions using the provided WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for interacting with the web page.
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError.
///
async fn should_make_paypal_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/234"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Makes a PayPal Sofort payment using the given web driver. This method initiates a series of events to simulate the process of making a PayPal Sofort payment, including redirection, clicking on specific elements, and asserting the presence of certain elements. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_paypal_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/235"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a unit test for the function `should_make_paypal_paypal_wallet_payment`.
fn should_make_paypal_paypal_wallet_payment_test() {
    tester!(should_make_paypal_paypal_wallet_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_paypal_ideal_payment function. It uses the tester! macro to run the test case for making a PayPal ideal payment.
fn should_make_paypal_ideal_payment_test() {
    tester!(should_make_paypal_ideal_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_paypal_giropay_payment method. It uses the tester! macro to execute the test and verify the functionality of the should_make_paypal_giropay_payment method.
fn should_make_paypal_giropay_payment_test() {
    tester!(should_make_paypal_giropay_payment);
}

#[test]
#[serial]
/// Calls the tester macro with the should_make_paypal_eps_payment function to test if a PayPal EPS payment can be successfully made.
fn should_make_paypal_eps_payment_test() {
    tester!(should_make_paypal_eps_payment);
}

#[test]
#[serial]
/// This method is a test case for making a PayPal Sofort payment. It utilizes the `tester!` macro to execute the test for making a PayPal Sofort payment.
fn should_make_paypal_sofort_payment_test() {
    tester!(should_make_paypal_sofort_payment);
}
