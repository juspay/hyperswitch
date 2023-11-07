use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct StripeSeleniumTest;

impl SeleniumTest for StripeSeleniumTest {
    fn get_connector_name(&self) -> String {
        "stripe".to_string()
    }
}

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
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
fn should_make_3ds_mandate_payment_test() {
    tester!(should_make_3ds_mandate_payment);
}

#[test]
#[serial]
fn should_fail_recurring_payment_due_to_authentication_test() {
    tester!(should_fail_recurring_payment_due_to_authentication);
}

#[test]
#[serial]
fn should_make_3ds_mandate_with_zero_dollar_payment_test() {
    tester!(should_make_3ds_mandate_with_zero_dollar_payment);
}

#[test]
#[serial]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
#[ignore]
fn should_make_gpay_mandate_payment_test() {
    tester!(should_make_gpay_mandate_payment);
}

#[test]
#[serial]
fn should_make_stripe_klarna_payment_test() {
    tester!(should_make_stripe_klarna_payment);
}

#[test]
#[serial]
fn should_make_afterpay_payment_test() {
    tester!(should_make_afterpay_payment);
}

#[test]
#[serial]
fn should_make_stripe_alipay_payment_test() {
    tester!(should_make_stripe_alipay_payment);
}

#[test]
#[serial]
fn should_make_stripe_ideal_bank_redirect_payment_test() {
    tester!(should_make_stripe_ideal_bank_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_giropay_bank_redirect_payment_test() {
    tester!(should_make_stripe_giropay_bank_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_eps_bank_redirect_payment_test() {
    tester!(should_make_stripe_eps_bank_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_bancontact_card_redirect_payment_test() {
    tester!(should_make_stripe_bancontact_card_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_p24_redirect_payment_test() {
    tester!(should_make_stripe_p24_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_sofort_redirect_payment_test() {
    tester!(should_make_stripe_sofort_redirect_payment);
}

#[test]
#[serial]
fn should_make_stripe_ach_bank_debit_payment_test() {
    tester!(should_make_stripe_ach_bank_debit_payment);
}

#[test]
#[serial]
fn should_make_stripe_sepa_bank_debit_payment_test() {
    tester!(should_make_stripe_sepa_bank_debit_payment);
}

#[test]
#[serial]
fn should_make_stripe_affirm_paylater_payment_test() {
    tester!(should_make_stripe_affirm_paylater_payment);
}
