use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct MollieSeleniumTest;

impl SeleniumTest for MollieSeleniumTest {
    fn get_connector_name(&self) -> String {
        "mollie".to_string()
    }
}

async fn should_make_mollie_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/32"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/29"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn: MollieSeleniumTest = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/36"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::ClassName(
                "payment-method-list--bordered",
            ))),
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

async fn should_make_mollie_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/38"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/41"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_bancontact_card_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/86"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_przelewy24_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/87"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Test profile")),
            Event::Trigger(Trigger::Click(By::Css("input[value='paid']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='button form__button']",
            ))),
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

async fn should_make_mollie_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = MollieSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/148"))),
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
fn should_make_mollie_paypal_payment_test() {
    tester!(should_make_mollie_paypal_payment);
}

#[test]
#[serial]
fn should_make_mollie_sofort_payment_test() {
    tester!(should_make_mollie_sofort_payment);
}

#[test]
#[serial]
fn should_make_mollie_ideal_payment_test() {
    tester!(should_make_mollie_ideal_payment);
}

#[test]
#[serial]
fn should_make_mollie_eps_payment_test() {
    tester!(should_make_mollie_eps_payment);
}

#[test]
#[serial]
fn should_make_mollie_giropay_payment_test() {
    tester!(should_make_mollie_giropay_payment);
}

#[test]
#[serial]
fn should_make_mollie_bancontact_card_payment_test() {
    tester!(should_make_mollie_bancontact_card_payment);
}

#[test]
#[serial]
fn should_make_mollie_przelewy24_payment_test() {
    tester!(should_make_mollie_przelewy24_payment);
}

#[test]
#[serial]
fn should_make_mollie_3ds_payment_test() {
    tester!(should_make_mollie_3ds_payment);
}
