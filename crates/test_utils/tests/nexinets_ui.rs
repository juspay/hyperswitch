use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct NexinetsSeleniumTest;

impl SeleniumTest for NexinetsSeleniumTest {
    fn get_connector_name(&self) -> String {
        "nexinets".to_string()
    }
}

async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/220"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a.btn.btn-primary.btn-block.margin-bottm-15",
            ))),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded", "status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_3ds_card_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/221"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("threeDSCReqIframe"))),
            Event::Trigger(Trigger::SendKeys(By::Id("otp"), "1234")),
            Event::Trigger(Trigger::Click(By::Css("button#sendOtp"))),
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

async fn should_make_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = NexinetsSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/222"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a.btn.btn-primary.btn-block.margin-bottm-15",
            ))),
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
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}

#[test]
#[serial]
fn should_make_3ds_card_payment_test() {
    tester!(should_make_3ds_card_payment);
}

#[test]
#[serial]
fn should_make_ideal_payment_test() {
    tester!(should_make_ideal_payment);
}
