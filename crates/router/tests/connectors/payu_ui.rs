use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct PayUSeleniumTest;

impl SeleniumTest for PayUSeleniumTest {
    fn get_connector_name(&self) -> String {
        "payu".to_string()
    }
}

async fn should_make_no_3ds_card_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = PayUSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/72"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(1)),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = PayUSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/77"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(1)),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_no_3ds_card_payment_test() {
    tester!(should_make_no_3ds_card_payment);
}

#[test]
#[serial]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
