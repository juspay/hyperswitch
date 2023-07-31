use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct NoonSeleniumTest;

impl SeleniumTest for NoonSeleniumTest {
    fn get_connector_name(&self) -> String {
        "noon".to_string()
    }
}

async fn should_make_noon_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/176"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("redirectTo3ds1Frame"))),
            Event::Trigger(Trigger::SwitchFrame(By::Css("iframe[frameborder='0']"))),
            Event::Trigger(Trigger::SendKeys(By::Css("input.input-field"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("input.button.primary"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_noon_3ds_mandate_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/214"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("redirectTo3ds1Frame"))),
            Event::Trigger(Trigger::SwitchFrame(By::Css("iframe[frameborder='0']"))),
            Event::Trigger(Trigger::SendKeys(By::Css("input.input-field"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("input.button.primary"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("a.btn"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_noon_non_3ds_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = NoonSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/215"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("a.btn"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_noon_3ds_payment_test() {
    tester!(should_make_noon_3ds_payment);
}

#[test]
#[serial]
fn should_make_noon_3ds_mandate_payment_test() {
    tester!(should_make_noon_3ds_mandate_payment);
}

#[test]
#[serial]
fn should_make_noon_non_3ds_mandate_payment_test() {
    tester!(should_make_noon_non_3ds_mandate_payment);
}
