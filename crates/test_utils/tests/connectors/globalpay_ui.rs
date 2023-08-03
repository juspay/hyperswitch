use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct GlobalpaySeleniumTest;

impl SeleniumTest for GlobalpaySeleniumTest {
    fn get_connector_name(&self) -> String {
        "globalpay".to_string()
    }
}

async fn should_make_gpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .globalpay_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?amount=10.00&country=US&currency=USD&gatewayname=globalpayments&gatewaymerchantid={pub_key}"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_globalpay_paypal_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_paypal_payment(
        driver,
        &format!("{CHEKOUT_BASE_URL}/saved/46"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("payment-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=succeeded"],
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_globalpay_ideal_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/53"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
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

async fn should_make_globalpay_giropay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/59"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
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

async fn should_make_globalpay_eps_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/50"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Choose your Bank")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Login to your Online Banking Account")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Transaction Authentication")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
            Event::Assert(Assert::IsPresent("Payment Successful")),
            Event::Trigger(Trigger::Click(By::Css("button.btn.btn-primary"))),
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

async fn should_make_globalpay_sofort_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = GlobalpaySeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/63"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::RunIf(
                Assert::IsPresent("Wählen"),
                vec![Event::Trigger(Trigger::Click(By::Css("p.description")))],
            ),
            Event::Assert(Assert::IsPresent("Demo Bank")),
            Event::Trigger(Trigger::SendKeys(
                By::Id("BackendFormLOGINNAMEUSERID"),
                "12345",
            )),
            Event::Trigger(Trigger::SendKeys(By::Id("BackendFormUSERPIN"), "1234")),
            Event::Trigger(Trigger::Click(By::Css(
                "button.button-right.primary.has-indicator",
            ))),
            Event::RunIf(
                Assert::IsPresent("Kontoauswahl"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button.button-right.primary.has-indicator",
                )))],
            ),
            Event::Assert(Assert::IsPresent("PPRO Payment Services Ltd.")),
            Event::Trigger(Trigger::SendKeys(By::Id("BackendFormTan"), "12345")),
            Event::Trigger(Trigger::Click(By::Css(
                "button.button-right.primary.has-indicator",
            ))),
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
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
fn should_make_globalpay_paypal_payment_test() {
    tester!(should_make_globalpay_paypal_payment);
}

#[test]
#[serial]
fn should_make_globalpay_ideal_payment_test() {
    tester!(should_make_globalpay_ideal_payment);
}

#[test]
#[serial]
fn should_make_globalpay_giropay_payment_test() {
    tester!(should_make_globalpay_giropay_payment);
}

#[test]
#[serial]
fn should_make_globalpay_eps_payment_test() {
    tester!(should_make_globalpay_eps_payment);
}

#[test]
#[serial]
fn should_make_globalpay_sofort_payment_test() {
    tester!(should_make_globalpay_sofort_payment);
}
