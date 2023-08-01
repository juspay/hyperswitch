use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct NuveiSeleniumTest;

impl SeleniumTest for NuveiSeleniumTest {
    fn get_connector_name(&self) -> String {
        "nuvei".to_string()
    }
}

async fn should_make_nuvei_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000027891380961&expmonth=10&expyear=25&cvv=123&amount=200&country=US&currency=USD"))),
            Event::Assert(Assert::IsPresent("Expiry Year")),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(Selector::Title, "ThreeDS ACS Emulator - Challenge Page")),
            Event::Trigger(Trigger::Click(By::Id("btn1"))),
            Event::Trigger(Trigger::Click(By::Id("btn5"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(Selector::QueryParamStr, "status=succeeded")),

    ]).await?;
    Ok(())
}

async fn should_make_nuvei_3ds_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000027891380961&expmonth=10&expyear=25&cvv=123&amount=200&country=US&currency=USD&setup_future_usage=off_session&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=in%20sit&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=7000&mandate_data[mandate_type][multi_use][currency]=USD&mandate_data[mandate_type][multi_use][start_date]=2022-09-10T00:00:00Z&mandate_data[mandate_type][multi_use][end_date]=2023-09-10T00:00:00Z&mandate_data[mandate_type][multi_use][metadata][frequency]=13&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(Selector::Title, "ThreeDS ACS Emulator - Challenge Page")),
            Event::Trigger(Trigger::Click(By::Id("btn1"))),
            Event::Trigger(Trigger::Click(By::Id("btn5"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("man_")),//mandate id prefix is present

    ]).await?;
    Ok(())
}

async fn should_make_nuvei_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=nuveidigital&gatewaymerchantid=googletest&amount=10.00&country=IN&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_nuvei_pypl_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_paypal_payment(
        c,
        &format!("{CHEKOUT_BASE_URL}/saved/5"),
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

async fn should_make_nuvei_giropay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/bank-redirect?amount=1.00&country=DE&currency=EUR&paymentmethod=giropay"))),
            Event::Trigger(Trigger::Click(By::Id("bank-redirect-btn"))),
            Event::Assert(Assert::IsPresent("You are about to make a payment using the Giropay service.")),
            Event::Trigger(Trigger::Click(By::Id("ctl00_ctl00_mainContent_btnConfirm"))),
            Event::RunIf(Assert::IsPresent("Bank suchen"), vec![
                Event::Trigger(Trigger::SendKeys(By::Id("bankSearch"), "GIROPAY Testbank 1")),
                Event::Trigger(Trigger::Click(By::Id("GIROPAY Testbank 1"))),
            ]),
            Event::Assert(Assert::IsPresent("GIROPAY Testbank 1")),
            Event::Trigger(Trigger::Click(By::Css("button[name='claimCheckoutButton']"))),
            Event::Assert(Assert::IsPresent("sandbox.paydirekt")),
            Event::Trigger(Trigger::Click(By::Id("submitButton"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Trigger(Trigger::SwitchTab(Position::Next)),
            Event::Assert(Assert::IsPresent("Sicher bezahlt!")),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(Selector::QueryParamStr, vec!["status=succeeded", "status=processing"]))
    ]).await?;
    Ok(())
}

async fn should_make_nuvei_ideal_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/bank-redirect?amount=10.00&country=NL&currency=EUR&paymentmethod=ideal&processingbank=ing"))),
            Event::Trigger(Trigger::Click(By::Id("bank-redirect-btn"))),
            Event::Assert(Assert::IsPresent("Your account will be debited:")),
            Event::Trigger(Trigger::SelectOption(By::Id("ctl00_ctl00_mainContent_ServiceContent_ddlBanks"), "ING Simulator")),
            Event::Trigger(Trigger::Click(By::Id("ctl00_ctl00_mainContent_btnConfirm"))),
            Event::Assert(Assert::IsPresent("IDEALFORTIS")),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Trigger(Trigger::Click(By::Id("ctl00_mainContent_btnGo"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(Selector::QueryParamStr, vec!["status=succeeded", "status=processing"]))
    ]).await?;
    Ok(())
}

async fn should_make_nuvei_sofort_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/bank-redirect?amount=10.00&country=DE&currency=EUR&paymentmethod=sofort"))),
            Event::Trigger(Trigger::Click(By::Id("bank-redirect-btn"))),
            Event::Assert(Assert::IsPresent("SOFORT")),
            Event::Trigger(Trigger::ChangeQueryParam("sender_holder", "John Doe")),
            Event::Trigger(Trigger::Click(By::Id("ctl00_mainContent_btnGo"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(Selector::QueryParamStr, vec!["status=succeeded", "status=processing"]))
    ]).await?;
    Ok(())
}

async fn should_make_nuvei_eps_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/bank-redirect?amount=10.00&country=AT&currency=EUR&paymentmethod=eps&processingbank=ing"))),
            Event::Trigger(Trigger::Click(By::Id("bank-redirect-btn"))),
            Event::Assert(Assert::IsPresent("You are about to make a payment using the EPS service.")),
            Event::Trigger(Trigger::SendKeys(By::Id("ctl00_ctl00_mainContent_ServiceContent_txtCustomerName"), "John Doe")),
            Event::Trigger(Trigger::Click(By::Id("ctl00_ctl00_mainContent_btnConfirm"))),
            Event::Assert(Assert::IsPresent("Simulator")),
            Event::Trigger(Trigger::SelectOption(By::Css("select[name='result']"), "Succeeded")),
            Event::Trigger(Trigger::Click(By::Id("submitbutton"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(Selector::QueryParamStr, vec!["status=succeeded", "status=processing"]))
    ]).await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_nuvei_3ds_payment_test() {
    tester!(should_make_nuvei_3ds_payment);
}

#[test]
#[serial]
fn should_make_nuvei_3ds_mandate_payment_test() {
    tester!(should_make_nuvei_3ds_mandate_payment);
}

#[test]
#[serial]
fn should_make_nuvei_gpay_payment_test() {
    tester!(should_make_nuvei_gpay_payment);
}

#[test]
#[serial]
fn should_make_nuvei_pypl_payment_test() {
    tester!(should_make_nuvei_pypl_payment);
}

#[test]
#[serial]
fn should_make_nuvei_giropay_payment_test() {
    tester!(should_make_nuvei_giropay_payment);
}

#[test]
#[serial]
fn should_make_nuvei_ideal_payment_test() {
    tester!(should_make_nuvei_ideal_payment);
}

#[test]
#[serial]
fn should_make_nuvei_sofort_payment_test() {
    tester!(should_make_nuvei_sofort_payment);
}

#[test]
#[serial]
fn should_make_nuvei_eps_payment_test() {
    tester!(should_make_nuvei_eps_payment);
}
