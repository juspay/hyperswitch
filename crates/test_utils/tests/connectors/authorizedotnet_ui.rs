use rand::Rng;
use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AuthorizedotnetSeleniumTest;

impl SeleniumTest for AuthorizedotnetSeleniumTest {
        /// Returns the connector name for the authorized.net payment gateway.
    fn get_connector_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}

/// Asynchronously makes a Google Pay (GPay) payment using the provided web driver. 
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver to use for making the payment
/// 
/// # Returns
/// 
/// Returns a Result indicating success or an error of type WebDriverError
/// 
async fn should_make_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizedotnetSeleniumTest {};
    let amount = rand::thread_rng().gen_range(1..1000); //This connector detects it as fradulent payment if the same amount is used for multiple payments so random amount is passed for testing
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .authorizedotnet_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=authorizenet&gatewaymerchantid={pub_key}&amount={amount}&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("status")),
        Event::Assert(Assert::IsPresent("processing")), // This connector status will be processing for one day
    ]).await?;
    Ok(())
}
/// Asynchronously makes a PayPal payment using the provided web driver. It triggers specific events based on the provided parameters to complete the payment process and waits for the payment to be processed. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizedotnetSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/156"),
        vec![
            Event::EitherOr(
                Assert::IsElePresent(By::Css(".reviewButton")),
                vec![Event::Trigger(Trigger::Click(By::Css(".reviewButton")))],
                vec![Event::Trigger(Trigger::Click(By::Id("payment-submit-btn")))],
            ),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")), // This connector status will be processing for one day
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
#[ignore]
/// This method is a test function for making a payment using Google Pay. It uses the `tester!` macro to execute the `should_make_gpay_payment` method.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_paypal_payment function. It uses the tester! macro to execute the should_make_paypal_payment function and verifies its behavior in making a PayPal payment.
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}
