use rand::Rng;
use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AuthorizedotnetSeleniumTest;

impl SeleniumTest for AuthorizedotnetSeleniumTest {
    fn get_connector_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}

async fn should_make_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizedotnetSeleniumTest {};
    let amount = rand::thread_rng().gen_range(1..1000); //This connector detects it as fradulent payment if the same amount is used for multiple payments so random amount is passed for testing
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .authorizedotnet_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(web_driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=authorizenet&gatewaymerchantid={pub_key}&amount={amount}&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("status")),
        Event::Assert(Assert::IsPresent("processing")), // This connector status will be processing for one day
    ]).await?;
    Ok(())
}

async fn should_make_paypal_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizedotnetSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/156"),
        vec![
            Event::EitherOr(
                Assert::IsElePresent(By::Css(".reviewButton")),
                vec![Event::Trigger(Trigger::Click(By::Css(".reviewButton")))],
                vec![Event::Trigger(Trigger::Click(By::Id("payment-submit-btn")))],
            ),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")), // This connector status will be processing for one day
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
#[ignore]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}

#[test]
#[serial]
fn should_make_paypal_payment_test() {
    tester!(should_make_paypal_payment);
}
