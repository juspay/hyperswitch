use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AirwallexSeleniumTest;

impl SeleniumTest for AirwallexSeleniumTest {
    fn get_connector_name(&self) -> String {
        "airwallex".to_string()
    }
}

async fn should_make_airwallex_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AirwallexSeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/85"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(
                Selector::Title,
                "Airwallex - Create 3D Secure Payment",
            )),
            Event::Trigger(Trigger::SendKeys(By::Id("challengeDataEntry"), "1234")),
            Event::Trigger(Trigger::Click(By::Id("submit"))),
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

async fn should_make_airwallex_gpay_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = AirwallexSeleniumTest {};
    let merchant_name = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .airwallex_merchant_name
        .unwrap();
    conn.make_gpay_payment(web_driver,
        &format!("{CHECKOUT_BASE_URL}/gpay?gatewayname=airwallex&gatewaymerchantid={merchant_name}&amount=70.00&country=US&currency=USD"),
        vec![
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(Selector::Title, "Airwallex - Create 3D Secure Payment")),
            Event::Trigger(Trigger::SendKeys(By::Id("challengeDataEntry"), "1234")),
            Event::Trigger(Trigger::Click(By::Id("submit"))),
            Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_airwallex_3ds_payment_test() {
    tester!(should_make_airwallex_3ds_payment);
}

#[test]
#[serial]
#[ignore]
fn should_make_airwallex_gpay_payment_test() {
    tester!(should_make_airwallex_gpay_payment);
}
