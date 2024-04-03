use rand::Rng;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AuthorizedotnetSeleniumTest;

impl SeleniumTest for AuthorizedotnetSeleniumTest {
    fn get_connector_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}

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
fn should_make_webhook_test() {
    tester!(should_make_webhook);
}
