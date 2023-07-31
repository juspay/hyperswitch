use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct PaypalSeleniumTest;

impl SeleniumTest for PaypalSeleniumTest {
    fn get_connector_name(&self) -> String {
        "paypal".to_string()
    }
}

async fn should_make_paypal_paypal_wallet_payment(
    web_driver: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = PaypalSeleniumTest {};
    conn.make_paypal_payment(
        web_driver,
        &format!("{CHEKOUT_BASE_URL}/saved/21"),
        vec![
            Event::Trigger(Trigger::Click(By::Css(".reviewButton"))),
            Event::Assert(Assert::IsPresent("How Search works")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=processing"],
            )),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
fn should_make_paypal_paypal_wallet_payment_test() {
    tester!(should_make_paypal_paypal_wallet_payment);
}
