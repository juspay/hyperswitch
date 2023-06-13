use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};
struct MollieSeleniumTest;
impl SeleniumTest for MollieSeleniumTest {
    fn get_connector_name(&self) -> String {
        "mollie".to_string()
    }
}

async fn should_make_mollie_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/148"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_mollie_3ds_payment_test() {
    tester!(should_make_mollie_3ds_payment);
}
