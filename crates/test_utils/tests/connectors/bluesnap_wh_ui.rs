#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]
use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct BluesnapSeleniumTest;

impl SeleniumTest for BluesnapSeleniumTest {
    fn get_connector_name(&self) -> String {
        "bluesnap".to_string()
    }
}

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
        &"succeeded",
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
