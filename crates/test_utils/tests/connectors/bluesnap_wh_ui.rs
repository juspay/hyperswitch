use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct BluesnapSeleniumTest;

impl SeleniumTest for BluesnapSeleniumTest {
        /// Retrieves the name of the connector, which is "bluesnap".
    fn get_connector_name(&self) -> String {
        "bluesnap".to_string()
    }
}

/// Asynchronously makes a webhook test using the specified web driver. It constructs a BluesnapSeleniumTest connection, initiates a webhook test using the provided WebDriver, URL, list of events, timeout, and success message, and awaits the result. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_webhook(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = BluesnapSeleniumTest {};
    conn.make_webhook_test(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/199"),
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
/// Executes the `should_make_webhook` tester macro, which tests whether the webhook should be created.
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
