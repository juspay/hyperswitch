use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct NexinetsSeleniumTest;

impl SeleniumTest for NexinetsSeleniumTest {
        /// Returns the name of the connector, which is "nexinets".
    fn get_connector_name(&self) -> String {
        "nexinets".to_string()
    }
}

/// This method initiates a PayPal payment process using the provided WebDriver. It makes a series of redirections and clicks on elements to simulate the user flow for making a PayPal payment. It then asserts that the URL contains either "status=succeeded" or "status=processing" to verify the success of the payment process.
async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/220"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a.btn.btn-primary.btn-block.margin-bottm-15",
            ))),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a 3DS card payment using the provided web driver. It initiates a series of events such as redirection, clicking on buttons, entering OTP, and asserting the presence of certain elements to complete the payment process. If successful, it returns a `Result` with no value, otherwise it returns a `WebDriverError`.
async fn should_make_3ds_card_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/221"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("threeDSCReqIframe"))),
            Event::Trigger(Trigger::SendKeys(By::Id("otp"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("button#sendOtp"))),
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

/// This method makes an ideal payment using the provided web driver. It initiates a series of events such as triggering a redirection, clicking on buttons, and asserting the presence of specific query parameters in the URL. It returns a Result indicating success or a WebDriverError in case of failure.
async fn should_make_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/222"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a.btn.btn-primary.btn-block.margin-bottm-15",
            ))),
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
/// This method is a test for making a PayPal payment. It uses the tester! macro to run the test for the should_make_paypal_payment method.
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}

#[test]
#[serial]
/// This method is a test case for making a 3DS (3D-Secure) card payment. It uses the `tester!` macro to run the `should_make_3ds_card_payment` method.
fn should_make_3ds_card_payment_test() {
    tester!(should_make_3ds_card_payment);
}

#[test]
#[serial]
/// This method is used to test the should_make_ideal_payment function. It uses the tester! macro to run the test case for the should_make_ideal_payment function.
fn should_make_ideal_payment_test() {
    tester!(should_make_ideal_payment);
}
