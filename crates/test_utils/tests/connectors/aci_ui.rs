use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AciSeleniumTest;

impl SeleniumTest for AciSeleniumTest {
    fn get_connector_name(&self) -> String {
        "aci".to_string()
    }
}

async fn should_make_aci_card_mandate_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/180"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("a.btn"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_alipay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/213"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("submit-success"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_interac_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/14"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("input[value='Continue payment']"))),
            Event::Trigger(Trigger::Click(By::Css("input[value='Confirm']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/208"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("input.button-body.button-short"))),
            Event::Trigger(Trigger::Click(By::Css("input.button-body.button-short"))),
            Event::Trigger(Trigger::Click(By::Css("input.button-body.button-short"))),
            Event::Trigger(Trigger::Click(By::Css("input.button-body.button-middle"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/211"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("input.pps-button"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/212"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.large.button.primary.expand.form-submitter",
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.large.button.primary.expand.form-submitter",
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.large.button.primary.expand.form-submitter",
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.large.button.primary.expand.form-submitter",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/209"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(By::Name("sc"), "10")),
            Event::Trigger(Trigger::SendKeys(By::Name("extensionSc"), "4000")),
            Event::Trigger(Trigger::SendKeys(By::Name("customerName1"), "Hopper")),
            Event::Trigger(Trigger::Click(By::Css("input[value='Absenden']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_trustly_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/13"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(2)),
            Event::Trigger(Trigger::Click(By::XPath(
                r#"//*[@id="app"]/div[1]/div/div[2]/div/ul/div[4]/div/div[1]/div[2]/div[1]/span"#,
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.sc-eJocfa.sc-oeezt.cDgdS.bptgBT",
            ))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input.sc-fXgAZx.hkChHq"),
                "123456789",
            )),
            Event::Trigger(Trigger::Click(By::Css(
                "button.sc-eJocfa.sc-oeezt.cDgdS.bptgBT",
            ))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input.sc-fXgAZx.hkChHq"),
                "783213",
            )),
            Event::Trigger(Trigger::Click(By::Css(
                "button.sc-eJocfa.sc-oeezt.cDgdS.bptgBT",
            ))),
            Event::Trigger(Trigger::Click(By::Css("div.sc-jJMGnK.laKGqb"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.sc-eJocfa.sc-oeezt.cDgdS.bptgBT",
            ))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input.sc-fXgAZx.hkChHq"),
                "355508",
            )),
            Event::Trigger(Trigger::Click(By::Css(
                "button.sc-eJocfa.sc-oeezt.cDgdS.bptgBT",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_aci_przelewy24_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/12"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("pf31"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.btn.btn-lg.btn-info.btn-block",
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.btn.btn-success.btn-lg.accept-button",
            ))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_aci_card_mandate_payment_test() {
    tester!(should_make_aci_card_mandate_payment);
}

#[test]
#[serial]
fn should_make_aci_alipay_payment_test() {
    tester!(should_make_aci_alipay_payment);
}

#[test]
#[serial]
fn should_make_aci_interac_payment_test() {
    tester!(should_make_aci_interac_payment);
}

#[test]
#[serial]
fn should_make_aci_eps_payment_test() {
    tester!(should_make_aci_eps_payment);
}

#[test]
#[serial]
fn should_make_aci_ideal_payment_test() {
    tester!(should_make_aci_ideal_payment);
}

#[test]
#[serial]
fn should_make_aci_sofort_payment_test() {
    tester!(should_make_aci_sofort_payment);
}

#[test]
#[serial]
fn should_make_aci_giropay_payment_test() {
    tester!(should_make_aci_giropay_payment);
}

#[test]
#[serial]
fn should_make_aci_trustly_payment_test() {
    tester!(should_make_aci_trustly_payment);
}

#[test]
#[serial]
fn should_make_aci_przelewy24_payment_test() {
    tester!(should_make_aci_przelewy24_payment);
}
