use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AdyenSeleniumTest;

impl SeleniumTest for AdyenSeleniumTest {
    fn get_connector_name(&self) -> String {
        "adyen".to_string()
    }
}

async fn should_make_adyen_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    let pub_key = conn.get_configs().adyen.unwrap().key1;
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid={pub_key}&amount=70.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("processing")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_gpay_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    let pub_key = conn.get_configs().adyen.unwrap().key1;
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid={pub_key}&amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=7000&mandate_data[mandate_type][multi_use][currency]=USD"),
        vec![
        Event::Assert(Assert::IsPresent("processing")),
        Event::Assert(Assert::IsPresent("Mandate ID")),
        Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
        Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
        Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
        Event::Assert(Assert::IsPresent("processing")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_gpay_zero_dollar_mandate_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    let pub_key = conn.get_configs().adyen.unwrap().key1;
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid={pub_key}&amount=0.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD"),
        vec![
        Event::Assert(Assert::IsPresent("processing")),
        Event::Assert(Assert::IsPresent("Mandate ID")),
        Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
        Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
        Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
        Event::Assert(Assert::IsPresent("processing")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_klarna_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(c,
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
            Event::Assert(Assert::IsPresent("processing")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
            Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
            Event::Assert(Assert::IsPresent("processing")),
    ]).await?;
    Ok(())
}

async fn should_make_adyen_paypal_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_paypal_payment(
        c,
        &format!("{CHEKOUT_BASE_URL}/paypal-redirect?amount=10.00&country=DE&currency=EUR"),
        vec![
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

async fn should_make_adyen_ach_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/58"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_sepa_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/51"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_bacs_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/54"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_ideal_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/52"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btnLink"))),
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

async fn should_make_adyen_eps_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/61"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
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

async fn should_make_adyen_blik_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/64"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_bancontact_card_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/68"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(By::Id("username"), "admin")),
            Event::Trigger(Trigger::SendKeys(By::Id("password"), "Juspay@123")),
            Event::Trigger(Trigger::Click(By::ClassName("button"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=failed"],
            )),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_wechatpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/75"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_mbway_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/64"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
        ],
    )
    .await?;
    Ok(())
}

async fn should_make_adyen_ebanking_fi_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/78"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("css-ns0tbt"))),
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

async fn should_make_adyen_onlinebanking_pl_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/79"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("user_account_pbl_correct"))),
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
fn should_make_adyen_paypal_payment_test() {
    tester!(should_make_adyen_paypal_payment);
}

#[test]
#[serial]
fn should_make_adyen_ach_payment_test() {
    tester!(should_make_adyen_ach_payment);
}

#[test]
#[serial]
fn should_make_adyen_sepa_payment_test() {
    tester!(should_make_adyen_sepa_payment);
}

#[test]
#[serial]
fn should_make_adyen_bacs_payment_test() {
    tester!(should_make_adyen_bacs_payment);
}

#[test]
#[serial]
fn should_make_adyen_ideal_payment_test() {
    tester!(should_make_adyen_ideal_payment);
}

#[test]
#[serial]
fn should_make_adyen_eps_payment_test() {
    tester!(should_make_adyen_eps_payment);
}

#[test]
#[serial]
fn should_make_adyen_blik_payment_test() {
    tester!(should_make_adyen_blik_payment);
}

#[test]
#[serial]
fn should_make_adyen_bancontact_card_payment_test() {
    tester!(should_make_adyen_bancontact_card_payment);
}

#[test]
#[serial]
fn should_make_adyen_wechatpay_payment_test() {
    tester!(should_make_adyen_wechatpay_payment);
}

#[test]
#[serial]
fn should_make_adyen_mbway_payment_test() {
    tester!(should_make_adyen_mbway_payment);
}

#[test]
#[serial]
fn should_make_adyen_ebanking_fi_payment_test() {
    tester!(should_make_adyen_ebanking_fi_payment);
}

#[test]
#[serial]
fn should_make_adyen_onlinebanking_pl_payment_test() {
    tester!(should_make_adyen_onlinebanking_pl_payment);
}
// https://hs-payments-test.netlify.app/paypal-redirect?amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_uFpxA0r6jjbVaxHSY3X0BZLL3erDUzvg3i51abwB1Bknu3fdiPxw475DQgnByn1z
