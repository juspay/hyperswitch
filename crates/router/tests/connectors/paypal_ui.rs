use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

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

#[test]
#[serial]
fn should_make_paypal_paypal_wallet_payment_test() {
    tester!(should_make_paypal_paypal_wallet_payment);
}
