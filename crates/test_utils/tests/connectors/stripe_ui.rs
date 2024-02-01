use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct StripeSeleniumTest;

impl SeleniumTest for StripeSeleniumTest {
        /// Returns the name of the connector, which in this case is "stripe".
    fn get_connector_name(&self) -> String {
            "stripe".to_string()
        }
}

/// Asynchronously makes a 3D Secure payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for making the payment
///
/// # Returns
///
/// Returns a `Result` indicating success or an error of type `WebDriverError`
///
async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000000000003063&expmonth=10&expyear=25&cvv=123&amount=100&country=US&currency=USD"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::Contains(Selector::QueryParamStr, "status=succeeded")),

    ]).await?;
    Ok(())
}


/// Makes a 3DS mandate payment using the provided WebDriver. This method simulates the process of making a 3DS mandate payment by interacting with the Stripe payment gateway through a WebDriver instance. It performs a series of events such as triggering redirection, clicking on specific elements, and asserting the presence of certain elements to complete the payment process. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_3ds_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002500003155&expmonth=10&expyear=25&cvv=123&amount=10&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

/// This method attempts to make a recurring payment using the provided WebDriver. It first creates a connection to StripeSeleniumTest, then proceeds to make a redirection payment by triggering a series of events such as clicking buttons, filling out forms, and asserting the presence of certain elements on the page. If the payment fails due to authentication requirements, a WebDriverError is returned. If the payment is successful, the method returns Ok(()).
async fn should_fail_recurring_payment_due_to_authentication(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002760003184&expmonth=10&expyear=25&cvv=123&amount=10&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Your card was declined. This transaction requires authentication.")),
    ]).await?;
    Ok(())
}

/// Makes a 3D Secure mandate with a zero dollar payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for the mandate creation process.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the WebDriver encounters an issue.
///
async fn should_make_3ds_mandate_with_zero_dollar_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002500003155&expmonth=10&expyear=25&cvv=123&amount=0&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Css("#pm-mandate-btn a"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            // Need to be handled as mentioned in https://stripe.com/docs/payments/save-and-reuse?platform=web#charge-saved-payment-method
            Event::Assert(Assert::IsPresent("succeeded")),

    ]).await?;
    Ok(())
}

/// This method initiates a Google Pay payment using the provided WebDriver. It retrieves the Stripe public key from the automation configurations and uses it to make the payment through the StripeSeleniumTest connection. The method constructs the payment URL and performs the payment using the provided WebDriver instance. It then awaits the result of the payment operation and returns a Result indicating success or an error of type WebDriverError.
async fn should_make_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .stripe_pub_key
        .unwrap();
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=stripe&gpaycustomfields[stripe:version]=2018-10-31&gpaycustomfields[stripe:publishableKey]={pub_key}&amount=70.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

/// This method initiates a Google Pay (GPay) payment using the given WebDriver instance. It first retrieves the Stripe public key from the automation configurations, and then constructs a GPay payment URL with specific parameters. It then performs a series of events using the WebDriver, such as making assertions, triggering clicks, and validating the payment process. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_gpay_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .stripe_pub_key
        .unwrap();
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=stripe&gpaycustomfields[stripe:version]=2018-10-31&gpaycustomfields[stripe:publishableKey]={pub_key}&amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD"),
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

#[ignore = "Different flows"]
//https://stripe.com/docs/testing#regulatory-cards
async fn should_make_stripe_klarna_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/19"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("klarna-apf-iframe"))),
            Event::RunIf(
                Assert::IsPresent("Let’s verify your phone"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("phone"), "8056594427")),
                    Event::Trigger(Trigger::Click(By::Id("onContinue"))),
                    Event::Trigger(Trigger::SendKeys(By::Id("otp_field"), "123456")),
                ],
            ),
            Event::RunIf(
                Assert::IsPresent("We've updated our Shopping terms"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button[data-testid='kaf-button']",
                )))],
            ),
            Event::RunIf(
                Assert::IsPresent("Pick a plan"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button[data-testid='pick-plan']",
                )))],
            ),
            Event::Trigger(Trigger::Click(By::Css(
                "button[data-testid='confirm-and-pay']",
            ))),
            Event::RunIf(
                Assert::IsPresent("Fewer clicks"),
                vec![Event::Trigger(Trigger::Click(By::Css(
                    "button[data-testid='SmoothCheckoutPopUp:skip']",
                )))],
            ),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
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

/// This method initiates a payment process using Afterpay on a given WebDriver. It creates a connection to a StripeSeleniumTest, makes a series of redirection payments by triggering specific events, and asserts the presence of Google and the status of the payment. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_afterpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/22"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// Asynchronously makes a payment using Stripe and Alipay through WebDriver.
///
/// # Arguments
///
/// * `c` - A WebDriver instance to interact with the browser
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError
async fn should_make_stripe_alipay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/35"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[class='common-Button common-Button--default']",
            ))),
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
/// Asynchronously makes a redirection payment using Stripe ideal bank redirect and WebDriver, and performs a series of events and assertions to complete the payment process.
async fn should_make_stripe_ideal_bank_redirect_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/2"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// Asynchronously makes a redirection payment using Stripe and Giropay bank redirect method.
///
/// # Arguments
///
/// * `c` - The WebDriver to interact with the browser.
///
/// # Returns
///
/// A Result indicating success or a WebDriverError if an error occurs.
///
async fn should_make_stripe_giropay_bank_redirect_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/1"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// Asynchronously makes a redirection payment using Stripe and EPS (Electronic Payment Standard) bank, using the provided WebDriver.
/// 
/// # Arguments
/// 
/// * `c` - The WebDriver to use for the payment process.
/// 
/// # Returns
/// 
/// * If successful, returns Ok(()), otherwise returns a WebDriverError.
/// 
async fn should_make_stripe_eps_bank_redirect_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/26"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// This method is used to simulate a Stripe Bancontact card redirect payment using a WebDriver. It makes a redirection payment by performing a series of events such as triggering a page navigation, clicking on specific elements, and asserting the presence of certain elements on the page. If the payment is successful, it returns Ok(()); otherwise, it returns a WebDriverError.
async fn should_make_stripe_bancontact_card_redirect_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/28"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// This method should make a redirection payment using the p24 method in Stripe. It uses the provided WebDriver to interact with the Stripe payment flow, including navigating to the checkout URL, clicking the card submit button, clicking the default payment method button, and asserting the presence of "Google" and the successful status of the payment. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_stripe_p24_redirect_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/31"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
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

/// Asynchronously makes a Stripe sofort redirect payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for making the payment
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error with the WebDriver
///
async fn should_make_stripe_sofort_redirect_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/34"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "a[class='common-Button common-Button--default']",
            ))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing", "status=succeeded"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Stripe ACH bank debit method through the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for making the payment
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the payment fails
///
async fn should_make_stripe_ach_bank_debit_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/56"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input[class='p-CodePuncher-controllingInput']"),
                "11AA",
            )),
            Event::Trigger(Trigger::Click(By::Css(
                "div[class='SubmitButton-IconContainer']",
            ))),
            Event::Assert(Assert::IsPresent("Thanks for your payment")),
            Event::Assert(Assert::IsPresent("You completed a payment")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Stripe SEPA bank debit payment using the provided WebDriver. 
/// 
/// # Arguments
/// 
/// * `c` - The WebDriver to use for making the payment.
/// 
/// # Returns
/// 
/// Returns a Result indicating success or an error of type WebDriverError.
/// 
async fn should_make_stripe_sepa_bank_debit_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/67"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a Stripe Affirm Paylater payment using the provided WebDriver. 
/// 
/// # Arguments
/// 
/// * `driver` - The WebDriver to use for making the payment
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the payment could not be made
/// 
async fn should_make_stripe_affirm_paylater_payment(
    driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_affirm_payment(
        driver,
        &format!("{CHEKOUT_BASE_URL}/saved/110"),
        vec![Event::Assert(Assert::IsPresent("succeeded"))],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a test for the should_make_3ds_payment function. It uses the tester! macro to check if the should_make_3ds_payment function behaves as expected.
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_3ds_mandate_payment function. 
/// It uses the tester macro to execute the should_make_3ds_mandate_payment function and verify its behavior.
fn should_make_3ds_mandate_payment_test() {
    tester!(should_make_3ds_mandate_payment);
}

#[test]
#[serial]
/// This method tests whether the recurring payment fails due to authentication issues.
fn should_fail_recurring_payment_due_to_authentication_test() {
    tester!(should_fail_recurring_payment_due_to_authentication);
}

#[test]
#[serial]
/// This method is a test case for making a 3DS mandate with a zero dollar payment. It uses the `tester!` macro to run the `should_make_3ds_mandate_with_zero_dollar_payment` method.
fn should_make_3ds_mandate_with_zero_dollar_payment_test() {
    tester!(should_make_3ds_mandate_with_zero_dollar_payment);
}

#[test]
#[serial]
/// This method is a test case for whether the should_make_gpay_payment function should make a Google Pay payment. It uses the tester macro to run the test.
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
#[ignore]
/// Executes a test to verify the functionality of making a Google Pay mandate payment.
fn should_make_gpay_mandate_payment_test() {
    tester!(should_make_gpay_mandate_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_stripe_klarna_payment function, which is responsible for making a payment using Klarna through Stripe. It uses the tester! macro to perform the test for the should_make_stripe_klarna_payment function.
fn should_make_stripe_klarna_payment_test() {
    tester!(should_make_stripe_klarna_payment);
}

#[test]
#[serial]
/// This method is a test function that checks if the afterpay payment should be made. It uses the tester macro to run the should_make_afterpay_payment function.
fn should_make_afterpay_payment_test() {
    tester!(should_make_afterpay_payment);
}

#[test]
#[serial]
/// This method is a test function for the should_make_stripe_alipay_payment method. 
/// It uses the tester! macro to run the test for making a payment using Stripe Alipay.
fn should_make_stripe_alipay_payment_test() {
    tester!(should_make_stripe_alipay_payment);
}

#[test]
#[serial]
/// This method is used to test the functionality of making a Stripe ideal bank redirect payment. 
fn should_make_stripe_ideal_bank_redirect_payment_test() {
    tester!(should_make_stripe_ideal_bank_redirect_payment);
}

#[test]
#[serial]
/// Calls the tester macro with the specified method should_make_stripe_giropay_bank_redirect_payment to test the functionality of making a stripe giropay bank redirect payment.
fn should_make_stripe_giropay_bank_redirect_payment_test() {
    tester!(should_make_stripe_giropay_bank_redirect_payment);
}

#[test]
#[serial]
/// This method is a test case for the `should_make_stripe_eps_bank_redirect_payment` function. It uses the `tester` macro to test the functionality of the `should_make_stripe_eps_bank_redirect_payment` function.
fn should_make_stripe_eps_bank_redirect_payment_test() {
    tester!(should_make_stripe_eps_bank_redirect_payment);
}

#[test]
#[serial]
/// This method tests the functionality of making a Stripe Bancontact card redirect payment.
fn should_make_stripe_bancontact_card_redirect_payment_test() {
    tester!(should_make_stripe_bancontact_card_redirect_payment);
}

#[test]
#[serial]
/// This function is a test for the should_make_stripe_p24_redirect_payment method. It uses the tester! macro to assert the behavior of the method.
fn should_make_stripe_p24_redirect_payment_test() {
    tester!(should_make_stripe_p24_redirect_payment);
}

#[test]
#[serial]
/// This method is a test function for the should_make_stripe_sofort_redirect_payment method. 
/// It calls the tester macro to run the test and verify that the method behaves as expected.
fn should_make_stripe_sofort_redirect_payment_test() {
    tester!(should_make_stripe_sofort_redirect_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_stripe_ach_bank_debit_payment method. 
/// It uses the tester! macro to run the test case and verify the behavior of the method.
fn should_make_stripe_ach_bank_debit_payment_test() {
    tester!(should_make_stripe_ach_bank_debit_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_stripe_sepa_bank_debit_payment method. It uses the tester! macro to run the test and verify that the method makes a payment using SEPA bank debit through Stripe.
fn should_make_stripe_sepa_bank_debit_payment_test() {
    tester!(should_make_stripe_sepa_bank_debit_payment);
}

#[test]
#[serial]
/// This method is a test function for checking the functionality of making a payment using Stripe and Affirm PayLater. It uses the `tester!` macro to run the test for the `should_make_stripe_affirm_paylater_payment` method.
fn should_make_stripe_affirm_paylater_payment_test() {
    tester!(should_make_stripe_affirm_paylater_payment);
}
