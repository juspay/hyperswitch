use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct ZenSeleniumTest;

impl SeleniumTest for ZenSeleniumTest {
    fn get_connector_name(&self) -> String {
        "zen".to_string()
    }
}

async fn should_make_zen_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let mycon = ZenSeleniumTest {};
    mycon
        .make_redirection_payment(
            web_driver,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/201"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(10)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
                Event::Assert(Assert::IsPresent("succeeded")),
            ],
        )
        .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_zen_3ds_payment_test() {
    tester!(should_make_zen_3ds_payment);
}
