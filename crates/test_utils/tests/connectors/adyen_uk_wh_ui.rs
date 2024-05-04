use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AdyenSeleniumTest;

impl SeleniumTest for AdyenSeleniumTest {
    fn get_connector_name(&self) -> String {
        "adyen_uk".to_string()
    }
}

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
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
