use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct TrustpaySeleniumTest;

impl SeleniumTest for TrustpaySeleniumTest {
    fn get_connector_name(&self) -> String {
        "trustpay_3ds".to_string()
    }
}

async fn should_make_trustpay_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = TrustpaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHECKOUT_BASE_URL}/saved/206"))),
            Event::Trigger(Trigger::Sleep(1)),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button.btn.btn-lg.btn-primary.btn-block",
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

#[test]
#[serial]
#[ignore]
fn should_make_trustpay_3ds_payment_test() {
    tester!(should_make_trustpay_3ds_payment);
}
