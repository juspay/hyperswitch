use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AdyenSeleniumTest;

impl SeleniumTest for AdyenSeleniumTest {
        /// Returns the name of the connector, which is "adyen_uk" in this case.
    fn get_connector_name(&self) -> String {
        "adyen_uk".to_string()
    }
}

/// Asynchronously makes a webhook test using the provided WebDriver. The method creates a new AdyenSeleniumTest connection and uses it to perform a webhook test on the specified URL with the given events and timeout. If the test is successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_webhook(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_webhook_test(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/104"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
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
/// This method is used to test the functionality of the should_make_webhook method by using the tester macro.
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
