use rand::Rng;
use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AuthorizedotnetSeleniumTest;

impl SeleniumTest for AuthorizedotnetSeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}

/// Asynchronously checks if a webhook should be made using the provided web driver. It creates a random amount for testing purposes to avoid detection as a fraudulent payment and then makes a webhook test using the specified web driver, URL, events to trigger and assert, and a timeout. If successful, it returns `Ok(())`, otherwise it returns a `WebDriverError`.
async fn should_make_webhook(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizedotnetSeleniumTest {};
    let amount = rand::thread_rng().gen_range(50..1000); //This connector detects it as fradulent payment if the same amount is used for multiple payments so random amount is passed for testing(
    conn.make_webhook_test(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/227?amount={amount}"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
        10,
        "processing",
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is used to test the should_make_webhook function using the tester macro.
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
