use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct Shift4SeleniumTest;

impl SeleniumTest for Shift4SeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "shift4".to_string()
    }
}

/// This method is used to make a 3DS payment using the provided WebDriver. It initiates a series of events to complete the payment process, including redirection to a specific URL, clicking on specified elements, and asserting the presence of certain elements and query parameters. If successful, it returns a Result with a unit value, otherwise it returns a WebDriverError.
async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/37"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
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

/// Asynchronously makes a Giropay payment using the provided WebDriver. The method triggers a series of events on the WebDriver to navigate to the Giropay payment page, click the necessary buttons, and assert the presence of certain elements. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_giropay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/39"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
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

/// Asynchronously performs a series of events to make an ideal payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for performing the payment events.
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError.
///
async fn should_make_ideal_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/42"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
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

/// This method is responsible for making a sofort payment using the provided WebDriver. It creates a Shift4SeleniumTest connection and uses it to make a series of redirection payments and assertions to complete the sofort payment process. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_sofort_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/43"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
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

/// Asynchronously makes a payment using Electronic Payment Solutions (EPS) through the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for the payment process.
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError.
///
async fn should_make_eps_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/157"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
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

#[test]
#[serial]
/// This method is a test case for checking if the 3DS payment is being made successfully. It uses the tester! macro to invoke the should_make_3ds_payment method and verify its functionality.
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
/// This method is a test case for the `should_make_giropay_payment` function. It uses the `tester!` macro to run the test for making a Giropay payment.
fn should_make_giropay_payment_test() {
    tester!(should_make_giropay_payment);
}

#[test]
#[serial]
/// This method is used to test the should_make_ideal_payment function.
/// It calls the tester macro with the should_make_ideal_payment function as an argument to perform the test.
fn should_make_ideal_payment_test() {
    tester!(should_make_ideal_payment);
}

#[test]
#[serial]
/// This method is a test for the `should_make_sofort_payment` function. It uses the `tester!` macro to run the test.
fn should_make_sofort_payment_test() {
    tester!(should_make_sofort_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_eps_payment function. It uses the tester macro to run the test case for making an EPS (Electronic Payment Standard) payment. 
fn should_make_eps_payment_test() {
    tester!(should_make_eps_payment);
}
