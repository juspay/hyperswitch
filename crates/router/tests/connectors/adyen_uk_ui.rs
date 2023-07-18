use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AdyenSeleniumTest;

impl SeleniumTest for AdyenSeleniumTest {
    fn get_connector_name(&self) -> String {
        "adyen_uk".to_string()
    }
}

async fn should_make_adyen_3ds_payment_failed(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/177"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("failed")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_3ds_payment_success(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/62"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
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

async fn should_make_adyen_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid=JuspayDEECOM&amount=70.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_gpay_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid=JuspayDEECOM&amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=7000&mandate_data[mandate_type][multi_use][currency]=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
        Event::Assert(Assert::IsPresent("Mandate ID")),
        Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
        Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
        Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_gpay_zero_dollar_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid=JuspayDEECOM&amount=0.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
        Event::Assert(Assert::IsPresent("Mandate ID")),
        Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
        Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
        Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_klarna_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/195"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("klarna-apf-iframe"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Trigger(Trigger::Click(By::Id("signInWithBankId"))),
            Event::Assert(Assert::IsPresent("Klart att betala")),
            Event::EitherOr(
                Assert::IsPresent("Klart att betala"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button[data-testid='confirm-and-pay']",
                )))],
                vec![
                    Event::Trigger(Trigger::Click(By::Css(
                        "button[data-testid='SmoothCheckoutPopUp:skip']",
                    ))),
                    Event::Trigger(Trigger::Click(By::Css(
                        "button[data-testid='confirm-and-pay']",
                    ))),
                ],
            ),
            Event::RunIf(
                Assert::IsPresent("Färre klick, snabbare betalning"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button[data-testid='SmoothCheckoutPopUp:enable']",
                )))],
            ),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")), // mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_alipay_hk_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/162"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::EitherOr(
                Assert::IsPresent("Payment Method Not Available"),
                vec![Event::Assert(Assert::IsPresent(
                    "Please try again or select a different payment method",
                ))],
                vec![
                    Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
                    Event::Assert(Assert::Contains(
                        Selector::QueryParamStr,
                        "status=succeeded",
                    )),
                ],
            ),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_bizum_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/186"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(By::Id("iPhBizInit"), "700000000")),
            Event::Trigger(Trigger::Click(By::Id("bBizInit"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn"))),
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

async fn should_make_adyen_clearpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_clearpay_payment(
        driver,
        &format!("{CHEKOUT_BASE_URL}/saved/163"),
        vec![
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

async fn should_make_adyen_twint_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/170"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
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

async fn should_make_adyen_dana_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/175"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(
                By::XPath("/html/body/div[1]/div[2]/div[1]/div/input"),
                "12345678901",
            )), // Mobile Number can be any random 11 digit number
            Event::Trigger(Trigger::Click(By::XPath("/html/body/div[1]/div[2]/button"))),
            Event::Trigger(Trigger::SendKeys(
                By::XPath("/html/body/div[1]/div[2]/div[1]/div[2]/div/input"),
                "111111",
            )), // PIN can be any random 11 digit number
            Event::Trigger(Trigger::Click(By::ClassName("btn-next"))),
            Event::Trigger(Trigger::Click(By::ClassName("btn-next"))),
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

#[test]
#[serial]
#[ignore]
fn should_make_adyen_gpay_payment_test() {
    tester!(should_make_adyen_gpay_payment);
}

#[test]
#[serial]
#[ignore]
fn should_make_adyen_gpay_mandate_payment_test() {
    tester!(should_make_adyen_gpay_mandate_payment);
}

#[test]
#[serial]
#[ignore]
fn should_make_adyen_gpay_zero_dollar_mandate_payment_test() {
    tester!(should_make_adyen_gpay_zero_dollar_mandate_payment);
}

#[test]
#[serial]
fn should_make_adyen_klarna_mandate_payment_test() {
    tester!(should_make_adyen_klarna_mandate_payment);
}

#[test]
#[serial]
fn should_make_adyen_3ds_payment_failed_test() {
    tester!(should_make_adyen_3ds_payment_failed);
}

#[test]
#[serial]
fn should_make_adyen_3ds_payment_success_test() {
    tester!(should_make_adyen_3ds_payment_success);
}

#[test]
#[serial]
fn should_make_adyen_alipay_hk_payment_test() {
    tester!(should_make_adyen_alipay_hk_payment);
}

#[test]
#[serial]
fn should_make_adyen_bizum_payment_test() {
    tester!(should_make_adyen_bizum_payment);
}

#[test]
#[serial]
fn should_make_adyen_clearpay_payment_test() {
    tester!(should_make_adyen_clearpay_payment);
}

#[test]
#[serial]
fn should_make_adyen_twint_payment_test() {
    tester!(should_make_adyen_twint_payment);
}

#[test]
#[serial]
fn should_make_adyen_dana_payment_test() {
    tester!(should_make_adyen_dana_payment);
}

// https://hs-payments-test.netlify.app/paypal-redirect?amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_uFpxA0r6jjbVaxHSY3X0BZLL3erDUzvg3i51abwB1Bknu3fdiPxw475DQgnByn1z
