use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct CheckoutSeleniumTest;

impl SeleniumTest for CheckoutSeleniumTest {
    fn get_connector_name(&self) -> String {
        "checkout".to_string()
    }
}

async fn should_make_frictionless_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/18"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Assert(Assert::IsPresent("Google Search")),
                Event::Trigger(Trigger::Sleep(5)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Assert(Assert::ContainsAny(
                    Selector::QueryParamStr,
                    vec!["status=succeeded", "status=processing"],
                )),
            ],
        )
        .await?;
    Ok(())
}

async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/20"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(10)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
                Event::Assert(Assert::IsPresent("Google")),
                Event::Assert(Assert::ContainsAny(
                    Selector::QueryParamStr,
                    vec!["status=succeeded", "status=processing"],
                )),
            ],
        )
        .await?;
    Ok(())
}

async fn should_make_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = CheckoutSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/73"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(10)), //url gets updated only after some time, so need this timeout to solve the issue
                Event::Trigger(Trigger::SwitchFrame(By::Name("cko-3ds2-iframe"))),
                Event::Trigger(Trigger::SendKeys(By::Id("password"), "Checkout1!")),
                Event::Trigger(Trigger::Click(By::Id("txtButton"))),
                Event::Assert(Assert::IsPresent("Google")),
                Event::Assert(Assert::ContainsAny(
                    Selector::QueryParamStr,
                    vec!["status=succeeded", "status=processing"],
                )),
            ],
        )
        .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_frictionless_3ds_payment_test() {
    tester!(should_make_frictionless_3ds_payment);
}

#[test]
#[serial]
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
#[ignore]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
