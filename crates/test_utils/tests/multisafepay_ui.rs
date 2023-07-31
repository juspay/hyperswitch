use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct MultisafepaySeleniumTest;

impl SeleniumTest for MultisafepaySeleniumTest {
    fn get_connector_name(&self) -> String {
        "multisafepay".to_string()
    }
}

async fn should_make_multisafepay_3ds_payment_success(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/207"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_multisafepay_3ds_payment_failed(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/93"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("failed")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/153"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[class='btn btn-default']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MultisafepaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/154"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='btn btn-msp-success btn-block']",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_multisafepay_3ds_payment_success_test() {
    tester!(should_make_multisafepay_3ds_payment_success);
}

#[test]
#[serial]
fn should_make_multisafepay_3ds_payment_failed_test() {
    tester!(should_make_multisafepay_3ds_payment_failed);
}

#[test]
#[serial]
#[ignore]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}
