use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct Shift4SeleniumTest;

impl SeleniumTest for Shift4SeleniumTest {
    fn get_connector_name(&self) -> String {
        "shift4".to_string()
    }
}

async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/37"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_giropay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/39"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_ideal_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/42"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_sofort_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/43"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_eps_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = Shift4SeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/44"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-success"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=succeeded",
            )),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
fn should_make_giropay_payment_test() {
    tester!(should_make_giropay_payment);
}

#[test]
#[serial]
fn should_make_ideal_payment_test() {
    tester!(should_make_ideal_payment);
}

#[test]
#[serial]
fn should_make_sofort_payment_test() {
    tester!(should_make_sofort_payment);
}

#[test]
#[serial]
fn should_make_eps_payment_test() {
    tester!(should_make_eps_payment);
}
