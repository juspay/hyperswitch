use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct StripeSeleniumTest;

impl SeleniumTest for StripeSeleniumTest {
        /// Returns the name of the connector, which in this case is "stripe".
    fn get_connector_name(&self) -> String {
        "stripe".to_string()
    }
}

/// Asynchronously makes a webhook test using the provided web driver. It creates a connection to StripeSeleniumTest and initiates a webhook test with the specified parameters including the URL, events, timeout, and expected outcome. If the test is successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_webhook(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_webhook_test(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/16"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
        10,
        "succeeded",
    )
    .await?;
    Ok(())
}

#[test]
#[serial]

/// This method is responsible for testing the functionality of the `should_make_webhook` method.
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
