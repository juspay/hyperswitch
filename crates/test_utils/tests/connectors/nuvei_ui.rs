use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct NuveiSeleniumTest;

impl SeleniumTest for NuveiSeleniumTest {
        /// Returns the name of the connector, which is "nuvei".
    fn get_connector_name(&self) -> String {
        "nuvei".to_string()
    }
}

/// Asynchronously makes a Nuvei 3DS payment using the provided WebDriver. This method simulates a series of events like redirection, form filling, button clicks, and assertions to complete the 3DS payment process on the Nuvei platform. If successful, it returns a Result containing an empty value, otherwise it returns a WebDriverError.
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

/// This method is used to make a Nuvei 3DS mandate payment using a given WebDriver. It creates a NuveiSeleniumTest connection and then makes a redirection payment by performing a series of events such as triggering a redirection, clicking on elements, querying for elements, and asserting the presence of specific elements. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
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

/// Asynchronously makes a Nuvei Google Pay payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver instance to use for making the payment.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - The result of the operation, empty if successful, containing a WebDriverError if an error occurred.
///
async fn should_make_nuvei_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = NuveiSeleniumTest {};
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=nuveidigital&gatewaymerchantid=googletest&amount=10.00&country=IN&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

/// This method is used to make a Nuvei PayPal payment through a WebDriver. It uses the provided WebDriver to interact with the Nuvei payment page, specifically to make a PayPal payment with the specified checkout URL and events to be triggered. It awaits the completion of the payment process and returns a result indicating success or a WebDriverError if an error occurs.
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

/// Makes a Nuvei Giropay payment using the provided WebDriver instance by simulating a series of events such as redirection, button clicks, assertion checks, and tab switching. Returns a Result indicating success or an error of type WebDriverError.
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

/// Should make a Nuvei ideal payment by simulating the necessary actions in the WebDriver.
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

/// This async method makes a Nuvei sofort payment using the provided WebDriver instance. 
/// It creates a NuveiSeleniumTest connection, then triggers a series of events to simulate the payment process, 
/// including redirection, clicking buttons, and asserting the presence of certain elements. 
/// If the payment process is successful, it returns Ok(()), otherwise it returns a WebDriverError.
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

/// This method is used to make a Nuvei EPS payment using the provided WebDriver instance. It performs a series of events such as redirection, clicking, asserting presence of elements, sending keys, selecting options, and making assertions based on the response. If all the events are successful, it returns Ok(()), indicating that the payment was made successfully. If any of the events fail, it returns a WebDriverError.
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
/// This method is a test for making a Nuvei 3DS payment. It uses the tester macro to run the should_make_nuvei_3ds_payment method.
fn should_make_nuvei_3ds_payment_test() {
    tester!(should_make_nuvei_3ds_payment);
}

#[test]
#[serial]
/// This method is a unit test for the should_make_nuvei_3ds_mandate_payment function. It uses the tester macro to run the test and verify that the function behaves as expected.
fn should_make_nuvei_3ds_mandate_payment_test() {
    tester!(should_make_nuvei_3ds_mandate_payment);
}

#[test]
#[serial]
/// This method is a test case for making a payment using the Nuvei GPay payment method.
fn should_make_nuvei_gpay_payment_test() {
    tester!(should_make_nuvei_gpay_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_nuvei_pypl_payment function, which is responsible for testing the functionality of making a payment using Nuvei and PayPal. It utilizes the tester! macro to run the test case.
fn should_make_nuvei_pypl_payment_test() {
    tester!(should_make_nuvei_pypl_payment);
}

#[test]
#[serial]
/// This method is a test function for making a Nuvei Giropay payment. It uses the tester macro to run the test for the should_make_nuvei_giropay_payment function.
fn should_make_nuvei_giropay_payment_test() {
    tester!(should_make_nuvei_giropay_payment);
}

#[test]
#[serial]
/// This method is used to test the functionality of making a Nuvei ideal payment. It uses the tester macro to run the test for the should_make_nuvei_ideal_payment function.
fn should_make_nuvei_ideal_payment_test() {
    tester!(should_make_nuvei_ideal_payment);
}

#[test]
#[serial]
/// Executes a test for making a Nuvei Sofort payment.
fn should_make_nuvei_sofort_payment_test() {
    tester!(should_make_nuvei_sofort_payment);
}

#[test]
#[serial]
/// This method is a test function for testing the `should_make_nuvei_eps_payment` method.
/// It uses the tester macro to run the test and verify the functionality of the `should_make_nuvei_eps_payment` method.
fn should_make_nuvei_eps_payment_test() {
    tester!(should_make_nuvei_eps_payment);
}
