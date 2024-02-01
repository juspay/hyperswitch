use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct CheckoutSeleniumTest;

impl SeleniumTest for CheckoutSeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "checkout".to_string()
    }
}

/// Asynchronously makes a frictionless 3DS payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - A WebDriver instance to use for making the payment
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError
///
async fn should_make_frictionless_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/18"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Assert(Assert::IsPresent("Google Search")),
                Event::Trigger(Trigger::Sleep(5)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Assert(Assert::ContainsAny(
                    Selector::QueryParamStr,
                    vec!["status=succeeded", "status=processing"],
                )),
            ],
        )
        .await?;
    Ok(())
}
/// This method is used to make a 3DS payment using the provided WebDriver. It triggers a series of events in the checkout process, including redirection, clicking on buttons, switching frames, sending keys, and asserting the presence of elements. It also includes sleep events to handle timing issues. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/20"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(5)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
                Event::Trigger(Trigger::Sleep(2)),
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

/// This method is used to make a Google Pay payment using a WebDriver. It performs a series of events such as redirection, clicking, switching frames, sending keys, and asserting element presence and content to complete the payment process. It returns a Result indicating success or a WebDriverError if an error occurs during the payment process.
async fn should_make_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/73"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(10)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
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
/// This method is a test case for making a frictionless 3DS payment. It uses the `tester!` macro to run the `should_make_frictionless_3ds_payment` test function.
fn should_make_frictionless_3ds_payment_test() {
    tester!(should_make_frictionless_3ds_payment);
}

#[test]
#[serial]
/// This method is a test case for making a 3DS payment using the `should_make_3ds_payment` function.
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
#[ignore]
/// Tests the functionality of making a Google Pay payment.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
