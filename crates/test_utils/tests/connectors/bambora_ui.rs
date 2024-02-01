use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct BamboraSeleniumTest;

impl SeleniumTest for BamboraSeleniumTest {
        /// Returns the name of the connector as a string.
    fn get_connector_name(&self) -> String {
        "bambora".to_string()
    }
}

/// Checks if a 3DS payment should be made using the provided WebDriver. 
///
/// # Arguments
///
/// * `c` - The WebDriver instance to use for making the payment.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error if the payment fails.
///
async fn should_make_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let mycon = BamboraSeleniumTest {};
    mycon
        .make_redirection_payment(
            c,
            vec![
                Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/33"))),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Click(By::Id("continue-transaction"))),
                Event::Assert(Assert::IsPresent("succeeded")),
            ],
        )
        .await?;
    Ok(())
}

#[test]
#[serial]
/// This method is a test case for the should_make_3ds_payment function. It uses the tester! macro to run the test for making a 3DS payment.
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}
