use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct MultisafepaySeleniumTest;

impl SeleniumTest for MultisafepaySeleniumTest {
    fn get_connector_name(&self) -> String {
        "multisafepay".to_string()
    }
}

async fn should_make_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!(
                "https://hs-payments-test.netlify.app/saved/153"
            ))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[class='btn btn-default']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
