use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct StripeSeleniumTest;

impl SeleniumTest for StripeSeleniumTest {}

async fn should_make_stripe_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
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

async fn should_make_stripe_3ds_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002500003155&expmonth=10&expyear=25&cvv=123&amount=10&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_DREFLPJC5SFpFBupKYovdCfg37xgM20g7oXVLQMHXP3t2kJMRSy6aof1rTe6tyyK&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
            Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
            Event::Assert(Assert::IsPresent("succeeded")),

    ]).await?;
    Ok(())
}

async fn should_fail_recurring_payment_due_to_authentication(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002760003184&expmonth=10&expyear=25&cvv=123&amount=10&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_DREFLPJC5SFpFBupKYovdCfg37xgM20g7oXVLQMHXP3t2kJMRSy6aof1rTe6tyyK&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
            Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
            Event::Assert(Assert::IsPresent("authentication_required: Your card was declined. This transaction requires authentication.")),

    ]).await?;
    Ok(())
}

async fn should_make_stripe_3ds_mandate_with_zero_dollar_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000002500003155&expmonth=10&expyear=25&cvv=123&amount=0&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD&apikey=dev_DREFLPJC5SFpFBupKYovdCfg37xgM20g7oXVLQMHXP3t2kJMRSy6aof1rTe6tyyK&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Id("test-source-authorize-3ds"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("Mandate ID")),
            Event::Assert(Assert::IsPresent("man_")),// mandate id starting with man_
            Event::Trigger(Trigger::Click(By::Id("pm-mandate-btn"))),
            Event::Trigger(Trigger::Click(By::Id("pay-with-mandate-btn"))),
            // Need to be handled as mentioned in https://stripe.com/docs/payments/save-and-reuse?platform=web#charge-saved-payment-method
            Event::Assert(Assert::IsPresent("succeeded")),

    ]).await?;
    Ok(())
}

async fn should_make_stripe_gpay_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=stripe&gpaycustomfields[stripe:version]=2018-10-31&gpaycustomfields[stripe:publishableKey]=pk_test_51Msk2GAGHc77EJXX78h549SX2uaOnEkUYqBfjcoD05PIpAnDkYxMn8nQ4d19im85NQuX4Z6WDyHaUw2fFTPBWsIY00Wa7oNerO&amount=70.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

async fn should_make_stripe_gpay_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = StripeSeleniumTest {};
    conn.make_gpay_payment(c,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=stripe&gpaycustomfields[stripe:version]=2018-10-31&gpaycustomfields[stripe:publishableKey]=pk_test_51Msk2GAGHc77EJXX78h549SX2uaOnEkUYqBfjcoD05PIpAnDkYxMn8nQ4d19im85NQuX4Z6WDyHaUw2fFTPBWsIY00Wa7oNerO&amount=70.00&country=US&currency=USD&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=127.0.0.1&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=700&mandate_data[mandate_type][multi_use][currency]=USD"),
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

//https://stripe.com/docs/testing#regulatory-cards

#[test]
#[serial]
fn should_make_stripe_3ds_payment_test() {
    tester!(should_make_stripe_3ds_payment);
}

#[test]
#[serial]
fn should_make_stripe_3ds_mandate_payment_test() {
    tester!(should_make_stripe_3ds_mandate_payment);
}

#[test]
#[serial]
fn should_fail_recurring_payment_due_to_authentication_test() {
    tester!(should_fail_recurring_payment_due_to_authentication);
}

#[test]
#[serial]
fn should_make_stripe_3ds_mandate_with_zero_dollar_payment_test() {
    tester!(should_make_stripe_3ds_mandate_with_zero_dollar_payment);
}

#[test]
#[serial]
fn should_make_stripe_gpay_payment_test() {
    tester!(should_make_stripe_gpay_payment);
}

#[test]
#[serial]
fn should_make_stripe_gpay_mandate_payment_test() {
    tester!(should_make_stripe_gpay_mandate_payment);
}
