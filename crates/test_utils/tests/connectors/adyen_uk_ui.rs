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
fn should_make_adyen_3ds_mandate_payment_test() {
    tester!(should_make_adyen_3ds_mandate_payment);
}

#[test]
#[serial]
fn should_make_adyen_3ds_mandate_with_zero_dollar_payment_test() {
    tester!(should_make_adyen_3ds_mandate_with_zero_dollar_payment);
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
fn should_make_adyen_swish_payment_test() {
    tester!(should_make_adyen_swish_payment);
}

#[test]
#[serial]
#[ignore = "Failing from connector side"]
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

#[ignore]
#[test]
#[serial]
fn should_make_adyen_giropay_payment_test() {
    tester!(should_make_adyen_giropay_payment);
}

#[ignore]
#[test]
#[serial]
fn should_make_adyen_walley_payment_test() {
    tester!(should_make_adyen_walley_payment);
}

#[test]
#[serial]
fn should_make_adyen_dana_payment_test() {
    tester!(should_make_adyen_dana_payment);
}

#[test]
#[serial]
fn should_make_adyen_blik_payment_test() {
    tester!(should_make_adyen_blik_payment);
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

#[ignore]
#[test]
#[serial]
fn should_make_adyen_momo_atm_payment_test() {
    tester!(should_make_adyen_momo_atm_payment);
}
