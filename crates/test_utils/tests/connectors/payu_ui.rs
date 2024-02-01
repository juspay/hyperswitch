use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct PayUSeleniumTest;

impl SeleniumTest for PayUSeleniumTest {
        /// Returns the name of the connector, which is "payu".
    fn get_connector_name(&self) -> String {
        "payu".to_string()
    }
}

/// This method is used to make a payment without using a 3DS card. It uses the given web driver to perform a series of events, including redirection to a specific URL, clicking a button, waiting for a brief period, and asserting the presence of certain elements on the page. If all events are successful, it returns Ok(()), indicating that the payment was made successfully.
async fn should_make_no_3ds_card_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PayUSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/72"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(1)),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Google Pay payment using the provided web driver.
///
/// # Arguments
///
/// * `web_driver` - The web driver to use for making the payment.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError.
///
async fn should_make_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PayUSeleniumTest {};
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=payu&gatewaymerchantid=459551&amount=70.00&country=US&currency=PLN"),
        vec![
        Event::Assert(Assert::IsPresent("Status")),
        Event::Assert(Assert::IsPresent("processing")),
    ]).await?;
    Ok(())
}

#[test]
#[serial]
/// This method tests the functionality of making a payment using a non-3D secure card.
fn should_make_no_3ds_card_payment_test() {
    tester!(should_make_no_3ds_card_payment);
}

#[test]
#[serial]
#[ignore]
/// This method is a test function to check the functionality of making a payment using Google Pay.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
