use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct PaypalSeleniumTest;

impl SeleniumTest for PaypalSeleniumTest {
    fn get_connector_name(&self) -> String {
        "paypal".to_string()
    }
}

async fn should_make_paypal_paypal_wallet_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/21"),
        vec![
            Event::Trigger(Trigger::Click(By::Css("#payment-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_paypal_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/181"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_paypal_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/233"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_paypal_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/234"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_paypal_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/235"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Name("Successful"))),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_paypal_paypal_wallet_payment_test() {
    tester!(should_make_paypal_paypal_wallet_payment);
}

#[test]
#[serial]
fn should_make_paypal_ideal_payment_test() {
    tester!(should_make_paypal_ideal_payment);
}

#[test]
#[serial]
fn should_make_paypal_giropay_payment_test() {
    tester!(should_make_paypal_giropay_payment);
}

#[test]
#[serial]
fn should_make_paypal_eps_payment_test() {
    tester!(should_make_paypal_eps_payment);
}

#[test]
#[serial]
fn should_make_paypal_sofort_payment_test() {
    tester!(should_make_paypal_sofort_payment);
}
