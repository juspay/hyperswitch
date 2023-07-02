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
            Event::Assert(Assert::IsPresent("Expiry Year")),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(Selector::QueryParamStr, "status=failed")),
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
            Event::Assert(Assert::IsPresent("Expiry Year")),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=processing",
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
async fn should_make_adyen_3ds_payment_failed(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/177"))),
            Event::Assert(Assert::IsPresent("Expiry Year")),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(Selector::QueryParamStr, "status=failed")),
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
            Event::Assert(Assert::IsPresent("Expiry Year")),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::Eq(Selector::Title, "Payment Authentication")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(
                Selector::QueryParamStr,
                "status=processing",
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
        Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
        Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
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
        Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
        Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_klarna_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/klarna-redirect?amount=70.00&country=SE&currency=SEK&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=SEK&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("klarna-redirect-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("klarna-apf-iframe"))),
            Event::Trigger(Trigger::Click(By::Id("signInWithBankId"))),
            Event::Assert(Assert::IsPresent("Klart att betala")),
            Event::EitherOr(Assert::IsPresent("Klart att betala"), vec![
                Event::Trigger(Trigger::Click(By::Css("button[data-testid='confirm-and-pay']"))),
            ],
            vec![
                Event::Trigger(Trigger::Click(By::Css("button[data-testid='SmoothCheckoutPopUp:skip']"))),
                Event::Trigger(Trigger::Click(By::Css("button[data-testid='confirm-and-pay']"))),
            ]
            ),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
            Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
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
                    " (Note: these error messages are not visible on the live platform) ",
                ))],
                vec![
                    Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
                    Event::Assert(Assert::IsPresent("succeeded")),
                ],
            ),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_online_banking_fpx_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/172"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_online_banking_thailand_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/184"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_touch_n_go_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/185"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_adyen_gpay_payment_test() {
    tester!(should_make_adyen_gpay_payment);
}

#[test]
#[serial]
fn should_make_adyen_gpay_mandate_payment_test() {
    tester!(should_make_adyen_gpay_mandate_payment);
}

#[test]
#[serial]
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
fn should_make_adyen_online_banking_fpx_payment_test() {
    tester!(should_make_adyen_online_banking_fpx_payment);
}

#[test]
#[serial]
fn should_make_adyen_online_banking_thailand_payment_test() {
    tester!(should_make_adyen_online_banking_thailand_payment);
}

#[test]
#[serial]
fn should_make_adyen_touch_n_go_payment_test() {
    tester!(should_make_adyen_touch_n_go_payment);
}

// https://hs-payments-test.netlify.app/paypal-redirect?amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_uFpxA0r6jjbVaxHSY3X0BZLL3erDUzvg3i51abwB1Bknu3fdiPxw475DQgnByn1z
