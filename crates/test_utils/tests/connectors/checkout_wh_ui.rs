use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct CheckoutSeleniumTest;

impl SeleniumTest for CheckoutSeleniumTest {
        /// Returns the name of the connector, which is "checkout".
    fn get_connector_name(&self) -> String {
            "checkout".to_string()
    }
}

/// Asynchronously makes a webhook test using the provided web driver. The method constructs a CheckoutSeleniumTest connection and invokes its make_webhook_test method with the specified parameters, including the web driver, URL, events, timeout, and expected result. If successful, it returns Ok(()); otherwise, it returns a WebDriverError.
async fn should_make_webhook(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = CheckoutSeleniumTest {};
    conn.make_webhook_test(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/18"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(8)),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
        10,
        "succeeded",
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is used to test the functionality of the should_make_webhook function. 
/// It calls the tester macro with should_make_webhook as the argument to perform the test.
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
