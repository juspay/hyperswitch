use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct NoonSeleniumTest;

impl SeleniumTest for NoonSeleniumTest {
    fn get_connector_name(&self) -> String {
        "noon".to_string()
    }
}

async fn should_make_noon_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(web_driver, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/176"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(5)),
            // Event::Trigger(Trigger::SendKeys(
            //     By::Css("input.input-field"),
            //     "1234",
            // )),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input[name='challengeDataEntry']"),
                "1234",
            )),
            Event::Trigger(Trigger::Click(
                By::Css("input.button.primary")
            )),
            // Event::Assert(Assert::IsPresent("Google")),
            // Event::Assert(Assert::Contains(Selector::QueryParamStr, "status=succeeded")),
            Event::Assert(Assert::IsPresent("succeeded")),

    ]).await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_noon_3ds_payment_test() {
    tester!(should_make_noon_3ds_payment);
}