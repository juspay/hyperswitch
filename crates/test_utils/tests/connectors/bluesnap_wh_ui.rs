use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

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
        "succeeded",
    )
    .await?;
    Ok(())
}

#[test]
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
