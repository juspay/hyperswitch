use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AdyenSeleniumTest;

impl SeleniumTest for AdyenSeleniumTest {
        /// This method returns the connector name "adyen_uk" as a String.
    fn get_connector_name(&self) -> String {
            "adyen_uk".to_string()
    }
}

/// Makes a payment using Adyen 3DS and expects it to fail. It performs a series of actions using the provided web driver, such as navigating to a specific URL, clicking on an element, asserting the presence of certain elements, entering text, and waiting for a specific amount of time. If the payment fails as expected, it returns Ok(()); otherwise, it returns a WebDriverError.
async fn should_make_adyen_3ds_payment_failed(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/177"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SwitchFrame(By::Name("threeDSIframe"))),
            Event::Assert(Assert::IsPresent("AUTHENTICATION DETAILS")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("failed")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen 3DS, and asserts that the payment is successful.
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver for interacting with the web page.
/// 
/// # Returns
/// 
/// Returns a Result indicating success or an error of type WebDriverError.
/// 
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
            Event::Assert(Assert::IsPresent("AUTHENTICATION DETAILS")),
            Event::Trigger(Trigger::SendKeys(By::ClassName("input-field"), "password")),
            Event::Trigger(Trigger::Click(By::Id("buttonSubmit"))),
            Event::Trigger(Trigger::Sleep(5)),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Makes a payment using Adyen 3DS mandate and WebDriver. It performs a series of events such as redirection, clicking, and assertion to complete the payment process.
async fn should_make_adyen_3ds_mandate_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/203"))),
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

/// This method initiates the process of making an Adyen 3DS mandate with a zero dollar payment using the provided web driver. It simulates a series of events such as redirection, clicking on buttons, and asserting the presence of specific elements to complete the mandate creation process.
async fn should_make_adyen_3ds_mandate_with_zero_dollar_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/204"))),
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

/// Asynchronously makes a payment using the Adyen gateway and Google Pay (GPay) on a web driver instance.
///
/// # Arguments
///
/// * `web_driver` - The web driver instance to use for making the payment.
///
/// # Returns
///
/// A `Result` indicating success or an error of type `WebDriverError`.
///
async fn should_make_adyen_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=adyen&gatewaymerchantid=JuspayDEECOM&amount=70.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}


/// This method initiates a GPay payment using the Adyen gateway and creates a mandate for future payments. It uses the provided web driver to navigate to the GPay payment page, fills in the necessary details, and performs the required actions to create the mandate. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
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

/// This method makes a zero-dollar mandate payment using Adyen gateway and Google Pay (GPay) as the payment method. It initiates the payment flow by accessing a specific URL with the required parameters, performs a series of events such as clicking buttons and asserting the presence of specific elements, and finally returns a result indicating the success or failure of the operation.
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

/// This method performs a series of actions using the provided web driver to make a payment through Adyen Klarna Mandate. It constructs a series of events to interact with the web page, including clicking, switching frames, waiting, asserting the presence of certain elements, and running conditional events based on the presence of specific elements. Once all the events are executed, it returns a Result indicating success or an error related to the web driver.
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

/// This method performs a payment process using Adyen and Alipay HK. It uses a provided web driver to navigate through the payment process, making redirections and clicking on specific elements on the page. It then handles different scenarios such as payment method not available or successful payment. If the payment process is successful, it returns Ok(()), otherwise it returns a WebDriverError.
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

/// Asynchronously makes a Bizum payment using Adyen through a WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for making the payment
///
/// # Returns
///
/// Returns a `Result` with a unit value or a `WebDriverError` if an error occurs during the payment process.
///
async fn should_make_adyen_bizum_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/186"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(By::Id("iPhBizInit"), "700000000")),
            Event::Trigger(Trigger::Click(By::Id("bBizInit"))),
            Event::Trigger(Trigger::Click(By::Css("input.btn.btn-lg.btn-continue"))),
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


/// Asynchronously makes a payment using Adyen Clearpay through the provided WebDriver.
async fn should_make_adyen_clearpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_clearpay_payment(
        driver,
        &format!("{CHEKOUT_BASE_URL}/saved/163"),
        vec![Event::Assert(Assert::IsPresent("succeeded"))],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a PayPal payment using Adyen WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for making the payment.
///
/// # Returns
///
/// Result indicating success or an error of type WebDriverError.
///
async fn should_make_adyen_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/202"),
        vec![
            Event::Trigger(Trigger::Click(By::Id("payment-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing"], //final status of this payment method will remain in processing state
            )),
        ],
    )
    .await?;
    Ok(())
}


/// Asynchronously makes an ACH payment using Adyen with the provided web driver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver used for the payment process.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError.
///
async fn should_make_adyen_ach_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/58"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}
/// This async function makes a SEPA payment using the Adyen payment provider. It takes a WebDriver as input and
/// performs a series of actions using the AdyenSeleniumTest connection to make the payment, including triggering a
/// redirection to a specific URL, clicking on a specific button, and asserting the presence of a specific element.
/// If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_adyen_sepa_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/51"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen BACS payment method by simulating user actions in a web browser.
/// 
/// # Arguments
/// 
/// * `web_driver` - The web driver to interact with the web page.
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the web driver encounters any issues.
/// 
async fn should_make_adyen_bacs_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/54"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Status")),
            Event::Assert(Assert::IsPresent("processing")), //final status of this payment method will remain in processing state
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes an Adyen iDeal payment using the provided web driver. This method
/// initiates a redirection payment process by simulating a series of browser events, such as
/// navigating to a specific URL, clicking on certain buttons, and asserting the presence of
/// a specific element. It returns a `Result` indicating success or a `WebDriverError` if an
/// error occurs during the process.
///
/// # Arguments
///
/// * `web_driver` - The web driver to use for the payment process.
///
async fn should_make_adyen_ideal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/52"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("btnLink"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen's Electronic Payment Standard (EPS) method.
/// 
/// # Arguments
/// 
/// * `web_driver` - The WebDriver to use for the payment process.
/// 
/// # Returns
/// 
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the WebDriver encounters a problem.
/// 
async fn should_make_adyen_eps_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/61"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen Bancontact card through a web driver.
///
/// # Arguments
/// * `web_driver` - An instance of WebDriver used for making the payment.
///
/// # Returns
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError.
///
async fn should_make_adyen_bancontact_card_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    let user = &conn
        .get_configs()
        .automation_configs
        .unwrap()
        .adyen_bancontact_username
        .unwrap();

    let pass = &conn
        .get_configs()
        .automation_configs
        .unwrap()
        .adyen_bancontact_pass
        .unwrap();

    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/68"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(By::Id("username"), user)),
            Event::Trigger(Trigger::SendKeys(By::Id("password"), pass)),
            Event::Trigger(Trigger::Click(By::ClassName("button"))),
            Event::Trigger(Trigger::Sleep(2)),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes an Adyen WeChatPay payment using the provided web driver.
///
/// # Arguments
///
/// * `web_driver` - The web driver to use for making the payment
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError
///
async fn should_make_adyen_wechatpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/75"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen's MBWay payment method by performing a series of events using the provided WebDriver. This method will navigate to a specific URL, click on a button, and assert the presence of certain elements to complete the payment process.
async fn should_make_adyen_mbway_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/196"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Status")),
            Event::Assert(Assert::IsPresent("processing")), //final status of this payment method will remain in processing state
        ],
    )
    .await?;
    Ok(())
}

/// This method makes a payment through Adyen eBanking FI by using the provided web driver. It performs a series of events and assertions to complete the payment process.
async fn should_make_adyen_ebanking_fi_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/78"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::ClassName("css-ns0tbt"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen online banking integration.
///
/// # Arguments
/// * `web_driver` - The web driver for interacting with the browser.
///
/// # Returns
/// * `Result<(), WebDriverError>` - A result indicating success or failure of the operation.
///
async fn should_make_adyen_onlinebanking_pl_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/197"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("user_account_pbl_correct"))),
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

#[ignore]
/// This method performs a series of actions using the AdyenSeleniumTest connection to make a Giropay payment through the given web driver. It simulates the user interacting with the web page by triggering events such as clicking buttons, sending keys, and asserting the presence of certain elements. After performing the actions, it awaits the result and returns Ok(()) if successful, or an error of type WebDriverError if any issues occur during the process.
async fn should_make_adyen_giropay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/70"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input[id='tags']"),
                "Testbank Fiducia 44448888 GENODETT488",
            )),
            Event::Trigger(Trigger::Click(By::Css("input[id='tags']"))),
            Event::Trigger(Trigger::Sleep(3)),
            Event::Trigger(Trigger::Click(By::Id("ui-id-3"))),
            Event::Trigger(Trigger::Click(By::ClassName("blueButton"))),
            Event::Trigger(Trigger::SendKeys(By::Name("sc"), "10")),
            Event::Trigger(Trigger::SendKeys(By::Name("extensionSc"), "4000")),
            Event::Trigger(Trigger::SendKeys(By::Name("customerName1"), "Hopper")),
            Event::Trigger(Trigger::SendKeys(
                By::Name("customerIBAN"),
                "DE36444488881234567890",
            )),
            Event::Trigger(Trigger::Click(By::Css("input[value='Absenden']"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing"], //final status of this payment method will remain in processing state
            )),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes an Adyen TWINT payment using the provided WebDriver. This method
/// initiates a redirection payment process using the AdyenSeleniumTest connection, and
/// performs a series of events including triggering a click on a specific button and
/// asserting the presence of a certain element. If successful, it returns Ok(()).
async fn should_make_adyen_twint_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/170"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css("button[value='authorised']"))),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// This method performs a series of events to make a payment using Adyen Walley. It uses the provided web driver to navigate to the checkout page, fill in payment details, and complete the transaction. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_adyen_walley_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/198"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Teknikmössor AB")),
            Event::Trigger(Trigger::SwitchFrame(By::ClassName(
                "collector-checkout-iframe",
            ))),
            Event::Trigger(Trigger::Click(By::Id("purchase"))),
            Event::Trigger(Trigger::Sleep(10)),
            Event::Trigger(Trigger::SwitchFrame(By::Css(
                "iframe[title='Walley Modal - idp-choices']",
            ))),
            Event::Assert(Assert::IsPresent("Identifisering")),
            Event::Trigger(Trigger::Click(By::Id("optionLoggInnMedBankId"))),
            Event::Trigger(Trigger::SwitchFrame(By::Css("iframe[title='BankID']"))),
            Event::Assert(Assert::IsPresent("Engangskode")),
            Event::Trigger(Trigger::SendKeys(By::Css("input[type='password']"), "otp")),
            Event::Trigger(Trigger::Sleep(4)),
            Event::Trigger(Trigger::Click(By::Css("button[title='Neste']"))),
            Event::Assert(Assert::IsPresent("Ditt BankID-passord")),
            Event::Trigger(Trigger::Sleep(4)),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input[type='password']"),
                "qwer1234",
            )),
            Event::Trigger(Trigger::Click(By::Css("button[title='Neste']"))),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a payment using Adyen Dana method through a series of interactions with the provided WebDriver.
///
/// # Arguments
///
/// * `driver` - WebDriver instance to interact with the web page
///
/// # Returns
///
/// Returns a Result indicating success or failure, where the Ok value is returned if the payment is successful, and the Err value contains a WebDriverError if any error occurs during the payment process.
///
async fn should_make_adyen_dana_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/175"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::SendKeys(
                By::Css("input[type='number']"),
                "12345678901",
            )), // Mobile Number can be any random 11 digit number
            Event::Trigger(Trigger::Click(By::Css("button"))),
            Event::Trigger(Trigger::SendKeys(By::Css("input[type='number']"), "111111")), // PIN can be any random 11 digit number
            Event::Trigger(Trigger::Click(By::ClassName("btn-next"))),
            Event::Trigger(Trigger::Sleep(3)),
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

/// This method makes a payment using Adyen online banking FPX method using the provided web driver. It creates a connection to AdyenSeleniumTest, then makes a redirection payment to the specified URL and performs a series of events including clicking on certain buttons and checking for the presence of a specific element. It returns a Result indicating success or an error of type WebDriverError.
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

/// This method is used to make an Adyen online banking payment in Thailand using a web driver. It initiates a series of events to simulate the payment process, including redirection to the payment page, clicking the card submit button, clicking the authorised button, and asserting the presence of the "succeeded" element. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
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

/// Asynchronously makes a payment using Adyen's Touch n Go redirection flow with the given web driver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver used to interact with the web page
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating the success or failure of the payment process
///
async fn should_make_adyen_touch_n_go_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/185"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
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

/// This method makes a payment using Adyen Swish by using the provided web driver to simulate the user interaction and perform necessary assertions. It constructs a sequence of events to navigate to the checkout URL, click the card submit button, and assert the presence of specific elements on the page. If the sequence of events is successful, it returns a result with an empty value. Otherwise, it returns a WebDriverError.
async fn should_make_adyen_swish_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/210"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
            Event::Assert(Assert::IsPresent("Next Action Type")),
            Event::Assert(Assert::IsPresent("qr_code_information")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes an Adyen BLIK payment using the provided WebDriver. This method initiates a redirection payment flow and performs a series of events such as triggering a click, asserting element presence, etc. If the payment is successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_adyen_blik_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/64"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Next Action Type")),
            Event::Assert(Assert::IsPresent("wait_screen_information")),
        ],
    )
    .await?;
    Ok(())
}

/// Performs a series of steps to make an Adyen MoMo ATM payment using the provided WebDriver.
///
/// # Arguments
///
/// * `web_driver` - The WebDriver to use for interacting with the web page.
///
/// # Returns
///
/// Returns a Result indicating success or an error of type WebDriverError.
///
async fn should_make_adyen_momo_atm_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AdyenSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/238"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Sleep(5)), // Delay for provider to not reject payment for botting
            Event::Trigger(Trigger::SendKeys(
                By::Id("card-number"),
                "9704 0000 0000 0018",
            )),
            Event::Trigger(Trigger::SendKeys(By::Id("card-expire"), "03/07")),
            Event::Trigger(Trigger::SendKeys(By::Id("card-name"), "NGUYEN VAN A")),
            Event::Trigger(Trigger::SendKeys(By::Id("number-phone"), "987656666")),
            Event::Trigger(Trigger::Click(By::Id("btn-pay-card"))),
            Event::Trigger(Trigger::SendKeys(By::Id("napasOtpCode"), "otp")),
            Event::Trigger(Trigger::Click(By::Id("napasProcessBtn1"))),
            Event::Trigger(Trigger::Sleep(5)), // Delay to get to status page
            Event::Assert(Assert::IsPresent("succeeded")),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
/// Calls the tester macro with the should_make_adyen_gpay_payment function to test if a payment can be made using Adyen GPay.
fn should_make_adyen_gpay_payment_test() {
    tester!(should_make_adyen_gpay_payment);
}

#[test]
#[serial]
/// This method is a test case for the should_make_adyen_gpay_mandate_payment function. 
/// It uses the tester macro to test the functionality of the should_make_adyen_gpay_mandate_payment function.
fn should_make_adyen_gpay_mandate_payment_test() {
    tester!(should_make_adyen_gpay_mandate_payment);
}

#[test]
#[serial]
/// This method is a test for making a zero dollar mandate payment using Adyen GPay. 
/// It uses the tester! macro to run the specific test should_make_adyen_gpay_zero_dollar_mandate_payment.
fn should_make_adyen_gpay_zero_dollar_mandate_payment_test() {
    tester!(should_make_adyen_gpay_zero_dollar_mandate_payment);
}

#[test]
#[serial]
/// This method is a test for making a payment using the Adyen Klarna mandate. 
/// It calls the tester macro with the should_make_adyen_klarna_mandate_payment function.
fn should_make_adyen_klarna_mandate_payment_test() {
    tester!(should_make_adyen_klarna_mandate_payment);
}

#[test]
#[serial]
/// This method is a test case for making a 3D Secure payment through Adyen and verifying that the payment fails. It uses the tester macro to run the should_make_adyen_3ds_payment_failed method.
fn should_make_adyen_3ds_payment_failed_test() {
    tester!(should_make_adyen_3ds_payment_failed);
}

#[test]
#[serial]
/// This method is a test function that checks if the should_make_adyen_3ds_mandate_payment
/// method is working as expected. It uses the tester! macro to invoke the method and verify
/// its functionality.
fn should_make_adyen_3ds_mandate_payment_test() {
    tester!(should_make_adyen_3ds_mandate_payment);
}

#[test]
#[serial]
/// This method is a test case for ensuring that the Adyen 3DS mandate is created successfully when a zero dollar payment is made. It uses the tester macro to execute the test case for the method should_make_adyen_3ds_mandate_with_zero_dollar_payment.
fn should_make_adyen_3ds_mandate_with_zero_dollar_payment_test() {
    tester!(should_make_adyen_3ds_mandate_with_zero_dollar_payment);
}

#[test]
#[serial]
/// This method is a test function that checks if the Adyen 3DS payment is successful. It uses the `tester!` macro to execute the test case for making the Adyen 3DS payment successful.
fn should_make_adyen_3ds_payment_success_test() {
    tester!(should_make_adyen_3ds_payment_success);
}

#[test]
#[serial]
/// This method is a test to check if the should_make_adyen_alipay_hk_payment function is working correctly.
fn should_make_adyen_alipay_hk_payment_test() {
    tester!(should_make_adyen_alipay_hk_payment);
}

#[test]
#[serial]
/// This method is a test case for making a payment using Adyen Swish. It uses the tester! macro to test the should_make_adyen_swish_payment method.
fn should_make_adyen_swish_payment_test() {
    tester!(should_make_adyen_swish_payment);
}

#[test]
#[serial]
#[ignore = "Failing from connector side"]
/// This method is a test function that checks if the payment using Adyen Bizum is successful.
fn should_make_adyen_bizum_payment_test() {
    tester!(should_make_adyen_bizum_payment);
}

#[test]
#[serial]
/// This method is used to test the functionality of making a payment using the Adyen Clearpay payment method.
fn should_make_adyen_clearpay_payment_test() {
    tester!(should_make_adyen_clearpay_payment);
}

#[test]
#[serial]
/// Calls the `should_make_adyen_twint_payment` test method using the `tester` macro to verify that the Adyen TWINT payment functionality is functioning as expected.
fn should_make_adyen_twint_payment_test() {
    tester!(should_make_adyen_twint_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_adyen_paypal_payment function, which is responsible for making a payment using the Adyen PayPal integration. It uses the tester! macro to run the test case for the should_make_adyen_paypal_payment function.
fn should_make_adyen_paypal_payment_test() {
    tester!(should_make_adyen_paypal_payment);
}

#[test]
#[serial]
/// This method is a test for making a payment using the Adyen ACH payment method. It uses the tester macro to run the test case "should_make_adyen_ach_payment".
fn should_make_adyen_ach_payment_test() {
    tester!(should_make_adyen_ach_payment);
}

#[test]
#[serial]
/// This method is a test function that checks if the system should make a payment using the Adyen SEPA payment method. It contains a tester macro that will run the should_make_adyen_sepa_payment function.
fn should_make_adyen_sepa_payment_test() {
    tester!(should_make_adyen_sepa_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_adyen_bacs_payment function. It uses the tester macro to check if the function should_make_adyen_bacs_payment returns the expected result.
fn should_make_adyen_bacs_payment_test() {
    tester!(should_make_adyen_bacs_payment);
}

#[test]
#[serial]
/// This method is a test function that tests the should_make_adyen_ideal_payment method. It is used to test whether the should_make_adyen_ideal_payment method is working as expected.
fn should_make_adyen_ideal_payment_test() {
    tester!(should_make_adyen_ideal_payment);
}

#[test]
#[serial]
/// This method is used to test the functionality of making a payment using Adyen EPS. It calls the tester macro to run the should_make_adyen_eps_payment test case.
fn should_make_adyen_eps_payment_test() {
    tester!(should_make_adyen_eps_payment);
}

#[test]
#[serial]
/// This method is a test function for making a payment using Adyen Bancontact card.
fn should_make_adyen_bancontact_card_payment_test() {
    tester!(should_make_adyen_bancontact_card_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_adyen_wechatpay_payment function, which is responsible for making a payment using the Adyen WeChat Pay service. It uses the tester macro to run the test case.
fn should_make_adyen_wechatpay_payment_test() {
    tester!(should_make_adyen_wechatpay_payment);
}

#[test]
#[serial]
/// This method is a test function for making a payment using Adyen's MB Way payment method.
fn should_make_adyen_mbway_payment_test() {
    tester!(should_make_adyen_mbway_payment);
}

#[test]
#[serial]
/// This method is a test for making an Adyen ebanking payment with the Finnish payment method.
fn should_make_adyen_ebanking_fi_payment_test() {
    tester!(should_make_adyen_ebanking_fi_payment);
}

#[test]
#[serial]
/// This function is a test case for making a payment using the Adyen online banking payment method for Poland. It uses the `tester!` macro to run the test for the `should_make_adyen_onlinebanking_pl_payment` method.
fn should_make_adyen_onlinebanking_pl_payment_test() {
    tester!(should_make_adyen_onlinebanking_pl_payment);
}

#[ignore]
#[test]
#[serial]
/// This method is a test function for making a payment using Adyen Giropay. It uses the tester! macro to test the should_make_adyen_giropay_payment function.
fn should_make_adyen_giropay_payment_test() {
    tester!(should_make_adyen_giropay_payment);
}

#[ignore]
#[test]
#[serial]
/// This method tests the functionality of making an Adyen wallet payment.
fn should_make_adyen_walley_payment_test() {
    tester!(should_make_adyen_walley_payment);
}

#[test]
#[serial]
/// This method is a test case for making a payment using Adyen Dana. It uses the tester macro to run the test for the should_make_adyen_dana_payment method.
fn should_make_adyen_dana_payment_test() {
    tester!(should_make_adyen_dana_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_adyen_blik_payment function. It uses the tester macro to run the test case and verify the behavior of the should_make_adyen_blik_payment function.
fn should_make_adyen_blik_payment_test() {
    tester!(should_make_adyen_blik_payment);
}

#[test]
#[serial]
/// This method is a test function to check if the adyen_online_banking_fpx_payment is made successfully.
fn should_make_adyen_online_banking_fpx_payment_test() {
    tester!(should_make_adyen_online_banking_fpx_payment);
}

#[test]
#[serial]
/// This method is a test function for making Adyen online banking payment in Thailand.
fn should_make_adyen_online_banking_thailand_payment_test() {
    tester!(should_make_adyen_online_banking_thailand_payment);
}

#[test]
#[serial]
/// This method is a test function that checks if the should_make_adyen_touch_n_go_payment function from the tester macro works as expected. It is used to verify the functionality of making a payment using the Adyen Touch n Go payment method.
fn should_make_adyen_touch_n_go_payment_test() {
    tester!(should_make_adyen_touch_n_go_payment);
}

#[ignore]
#[test]
#[serial]
/// This method is a test function to check if the payment using Adyen and MoMo ATM is successful.
fn should_make_adyen_momo_atm_payment_test() {
    tester!(should_make_adyen_momo_atm_payment);
}
