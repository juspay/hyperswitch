use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct BluesnapSeleniumTest;

impl SeleniumTest for BluesnapSeleniumTest {
    fn get_connector_name(&self) -> String {
        "bluesnap".to_string()
    }
}

async fn should_make_3ds_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = BluesnapSeleniumTest {};
    conn.make_redirection_payment(
        driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/200"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::RunIf(
                Assert::IsElePresent(By::Id("Cardinal-CCA-IFrame")),
                vec![
                    Event::Trigger(Trigger::SwitchFrame(By::Id("Cardinal-CCA-IFrame"))),
                    Event::Assert(Assert::IsPresent("Enter your code below")),
                    Event::Trigger(Trigger::SendKeys(By::Name("challengeDataEntry"), "1234")),
                    Event::Trigger(Trigger::Click(By::ClassName("button.primary"))),
                ],
            ),
            Event::Trigger(Trigger::Sleep(10)),
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

async fn should_make_gpay_payment(driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = BluesnapSeleniumTest {};
    let pub_key = conn
        .get_configs()
        .automation_configs
        .unwrap()
        .bluesnap_gateway_merchant_id
        .unwrap();
    conn.make_gpay_payment(driver,
        &format!("{CHEKOUT_BASE_URL}/gpay?gatewayname=bluesnap&gatewaymerchantid={pub_key}&amount=11.00&country=US&currency=USD"),
        vec![
        Event::Assert(Assert::IsPresent("succeeded")),
    ]).await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}

#[test]
#[serial]
fn should_make_gpay_payment_test() {
    tester!(should_make_gpay_payment);
}
