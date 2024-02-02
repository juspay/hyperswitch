use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct GlobalpaySeleniumTest;

impl SeleniumTest for GlobalpaySeleniumTest {
        /// This method returns the name of the connector as a string.
    fn get_connector_name(&self) -> String {
        "globalpay".to_string()
    }
}

/// Asynchronously makes a Google Pay payment using the provided WebDriver. 
///
/// # Arguments
///
/// * `driver` - The WebDriver to use for making the payment
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError
///
async fn should_make_gpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .globalpay_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?amount=10.00&country=US&currency=USD&gatewayname=globalpayments&gatewaymerchantid={pub_key}"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}
/// Asynchronously makes a payment through Globalpay using the PayPal method.
/// 
/// # Arguments
/// 
/// * `driver` - The WebDriver to interact with the browser.
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error from the WebDriver.
/// 
async fn should_make_globalpay_paypal_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_paypal_payment(
        driver,
        &format!("{CHEKOUT_BASE_URL}/saved/46"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("payment-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Globalpay ideal payment using the provided WebDriver. This method
/// performs a series of events and assertions to complete the payment process and waits for
/// the payment to be successful or in processing state. If the payment process encounters an
/// error, it will return a WebDriverError.
async fn should_make_globalpay_ideal_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/53"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Globalpay GiroPay payment using the provided WebDriver.
/// Returns a Result indicating success or an error of type WebDriverError.
async fn should_make_globalpay_giropay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/59"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using the Globalpay EPS service through the provided WebDriver.
///
/// # Arguments
/// * `driver` - The WebDriver to use for making the payment
///
/// # Returns
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError
///
async fn should_make_globalpay_eps_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/50"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// This method is responsible for making a Globalpay Sofort payment using the provided WebDriver. It performs a series of actions to simulate the payment process, including redirection, clicking on elements, sending keys, and asserting the presence of specific elements. If the payment process is successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_globalpay_sofort_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/63"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::RunIf(
                Assert::IsPresent("Wählen"),
                vec![Event::Trigger(Trigger::Click(By::Css("p.description")))],
            ),
            Event::Assert(Assert::IsPresent("Demo Bank")),
            Event::Trigger(Trigger::SendKeys(
                By::Id("BackendFormLOGINNAMEUSERID"),
                "12345",
            )),
            Event::Trigger(Trigger::SendKeys(By::Id("BackendFormUSERPIN"), "1234")),
            Event::Trigger(Trigger::Click(By::Css(
                "button.button-right.primary.has-indicator",
            ))),
            Event::RunIf(
                Assert::IsPresent("Kontoauswahl"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button.button-right.primary.has-indicator",
                )))],
            ),
            Event::Assert(Assert::IsPresent("PPRO Payment Services Ltd.")),
            Event::Trigger(Trigger::SendKeys(By::Id("BackendFormTan"), "12345")),
            Event::Trigger(Trigger::Click(By::Css(
                "button.button-right.primary.has-indicator",
            ))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a test function for making a payment using Google Pay. It utilizes the tester! macro to test the implementation of should_make_gpay_payment.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
/// Calls the `should_make_globalpay_paypal_payment` function from the `tester` macro to test the functionality of making a PayPal payment using GlobalPay.
fn should_make_globalpay_paypal_payment_test() {
    tester!(should_make_globalpay_paypal_payment);
}

#[test]
#[serial]
/// This method is used to perform a test for making a payment using Globalpay Ideal. It calls the tester macro with the should_make_globalpay_ideal_payment test case.
fn should_make_globalpay_ideal_payment_test() {
    tester!(should_make_globalpay_ideal_payment);
}

#[test]
#[serial]
/// This method is a unit test for the should_make_globalpay_giropay_payment function. It uses the tester! macro to call the function and check if the payment is made successfully using the GlobalPay GiroPay method.
fn should_make_globalpay_giropay_payment_test() {
    tester!(should_make_globalpay_giropay_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_globalpay_eps_payment function. It uses the tester macro to run the test and verify that the globalpay_eps_payment function behaves as expected.
fn should_make_globalpay_eps_payment_test() {
    tester!(should_make_globalpay_eps_payment);
}

#[test]
#[serial]
/// Executes a test to verify if the should_make_globalpay_sofort_payment function makes a payment using GlobalPay Sofort.
fn should_make_globalpay_sofort_payment_test() {
    tester!(should_make_globalpay_sofort_payment);
}
