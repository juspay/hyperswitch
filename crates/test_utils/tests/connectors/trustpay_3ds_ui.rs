use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct TrustpaySeleniumTest;

impl SeleniumTest for TrustpaySeleniumTest {
        /// Retrieves the name of the connector, which is "trustpay_3ds".
    fn get_connector_name(&self) -> String {
        "trustpay_3ds".to_string()
    }
}

/// Asynchronously makes a Trustpay 3DS payment using the provided web driver. This method initiates a series of events to complete the payment process, including redirection, clicking buttons, and asserting the presence of certain elements. If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_trustpay_3ds_payment(web_driver: WebDriver) -> Result<(), WebDriverError> {
    let conn = TrustpaySeleniumTest {};
    conn.make_redirection_payment(
        web_driver,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/206"))),
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
/// This method is a test for the should_make_trustpay_3ds_payment function. It uses the tester macro to run the test case should_make_trustpay_3ds_payment.
fn should_make_trustpay_3ds_payment_test() {
    tester!(should_make_trustpay_3ds_payment);
}
