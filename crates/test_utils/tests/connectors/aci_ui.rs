use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AciSeleniumTest;

impl SeleniumTest for AciSeleniumTest {
        /// Retrieves the name of the connector.
    /// 
    /// # Returns
    /// 
    /// A `String` containing the name of the connector, which is "aci".
    fn get_connector_name(&self) -> String {
        "aci".to_string()
    }
}

/// Asynchronously makes a payment for the ACI card mandate using the provided web driver.
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver to use for making the payment
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the WebDriver encounters an issue
/// 
async fn should_make_aci_card_mandate_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/180"))),
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

/// This method initiates a payment process using Alipay on the ACI platform by simulating a series of user interactions and assertions using the provided web driver. It first creates a connection to the ACI Selenium test environment, then performs a series of events such as triggering a page redirection, clicking on specific elements, and asserting the presence of certain elements to complete the Alipay payment process. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_aci_alipay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/213"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("submit-success"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Makes an ACI interac payment using the provided web driver. This method creates a connection to AciSeleniumTest and makes a series of redirection payments to complete the interac payment process.
async fn should_make_aci_interac_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/14"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("input[value='Continue payment']")),
            Event::Trigger(Trigger::Click(By::Css("input[value='Confirm']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Makes an ACI EPS payment using the given web driver. This method triggers a series of events to
/// complete the payment process and asserts that the payment has succeeded.
async fn should_make_aci_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/208"))),
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

/// Asynchronously makes an ACI ideal payment using the provided web driver. This method
/// sends a series of events to the web driver in order to simulate the process of making
/// a payment using ACI ideal, including redirection, clicking buttons, and asserting
/// the presence of a specific element. If successful, it returns Ok(()), otherwise it
/// returns a WebDriverError.
async fn should_make_aci_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/211"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("input.pps-button"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes an ACI Sofort payment using the provided web driver. This method
/// simulates a series of events and assertions to complete the payment process and returns
/// a Result indicating success or a WebDriverError if an error occurs.
async fn should_make_aci_sofort_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/212"))),
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

/// Asynchronously makes an ACI GiroPay payment using the provided WebDriver. 
/// It initiates a series of events to simulate the payment process, such as redirection, clicking buttons, 
/// and entering payment details. It then asserts the presence of a "succeeded" message to ensure 
/// the payment was successful. 
async fn should_make_aci_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/209"))),
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

/// Asynchronously makes a Trustly payment using ACI by performing a series of Selenium WebDriver events
/// and assertions to complete the payment process. Returns a Result indicating success or an error of type
/// WebDriverError.
async fn should_make_aci_trustly_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/13"))),
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

/// Asynchronously makes a payment using Przelewy24 through ACI, using the provided web driver. 
/// It initiates a series of events and assertions to complete the payment process.
async fn should_make_aci_przelewy24_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AciSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/12"))),
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
/// This method is a test for the should_make_aci_card_mandate_payment function.
fn should_make_aci_card_mandate_payment_test() {
    tester!(should_make_aci_card_mandate_payment);
}

#[test]
#[serial]
/// This method is a test function to check if the should_make_aci_alipay_payment method works as expected. It uses the tester! macro to perform the test.
fn should_make_aci_alipay_payment_test() {
    tester!(should_make_aci_alipay_payment);
}

#[test]
#[serial]
/// This function is a test case for the should_make_aci_interac_payment function. 
/// It uses the tester! macro to run the specified function and compare the expected result with the actual result.
fn should_make_aci_interac_payment_test() {
    tester!(should_make_aci_interac_payment);
}

#[test]
#[serial]
/// This method is a unit test for the `should_make_aci_eps_payment` function. It uses the `tester!` macro to run the test and verify that the `should_make_aci_eps_payment` function behaves as expected.
fn should_make_aci_eps_payment_test() {
    tester!(should_make_aci_eps_payment);
}

#[test]
#[serial]
/// This method is used to test the function should_make_aci_ideal_payment. 
fn should_make_aci_ideal_payment_test() {
    tester!(should_make_aci_ideal_payment);
}

#[test]
#[serial]
/// This method is a test function that checks whether the `should_make_aci_sofort_payment` method behaves as expected. It uses the `tester!` macro to run the test case for making a payment using sofort method.
fn should_make_aci_sofort_payment_test() {
    tester!(should_make_aci_sofort_payment);
}

#[test]
#[serial]
/// Executes the `should_make_aci_giropay_payment` test using the `tester` macro.
fn should_make_aci_giropay_payment_test() {
    tester!(should_make_aci_giropay_payment);
}

#[test]
#[serial]
/// This method is a test function for the should_make_aci_trustly_payment method. It uses the tester! macro to run the test and verify that the should_make_aci_trustly_payment method behaves as expected.
fn should_make_aci_trustly_payment_test() {
    tester!(should_make_aci_trustly_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_aci_przelewy24_payment function, which is used to test the functionality of making a payment using the ACI Przelewy24 payment method.
fn should_make_aci_przelewy24_payment_test() {
    tester!(should_make_aci_przelewy24_payment);
}
