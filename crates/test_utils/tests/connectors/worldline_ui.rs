use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct WorldlineSeleniumTest;

impl SeleniumTest for WorldlineSeleniumTest {
        /// Returns the name of the connector as a String.
    fn get_connector_name(&self) -> String {
        "worldline".to_string()
    }
}

/// Asynchronously makes a non-3DS payment using the provided WebDriver.
///
/// # Arguments
///
/// * `c` - The WebDriver to use for making the payment.
///
/// # Returns
///
/// * `Result<(), WebDriverError>` - A result indicating success or an error of type WebDriverError.
///
async fn should_make_card_non_3ds_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = WorldlineSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/71"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("status")),
            Event::Assert(Assert::IsPresent("processing")),
        ],
    )
    .await?;
    Ok(())
}

/// Asynchronously makes a redirection payment using the WorldlineSeleniumTest connection, 
/// and a WebDriver instance. It creates a sequence of events including triggering a goto 
/// action, clicking a specific button, and asserting the presence of certain elements. 
/// If successful, it returns Ok(()), otherwise it returns a WebDriverError.
async fn should_make_worldline_ideal_redirect_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = WorldlineSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/49"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=requires_customer_action", "status=succeeded"],
            )),
        ],
    )
    .await?;
    Ok(())
}

/// This method is used to make a worldline giropay redirect payment using a WebDriver. It creates a connection to WorldlineSeleniumTest and then makes a redirection payment by performing a series of events such as triggering a URL, clicking a button, and asserting the presence of certain elements. If successful, it returns Ok, otherwise it returns a WebDriverError.
async fn should_make_worldline_giropay_redirect_payment(
    c: WebDriver,
) -> Result<(), WebDriverError> {
    let conn = WorldlineSeleniumTest {};
    conn.make_redirection_payment(
        c,
        vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/saved/48"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Assert(Assert::IsPresent("Google")),
            Event::Assert(Assert::ContainsAny(
                Selector::QueryParamStr,
                vec!["status=requires_customer_action", "status=succeeded"],
            )),
        ],
    )
    .await?;
    Ok(())
}

#[test]
#[serial]
#[ignore]
/// This method tests the functionality of making a Worldline Giropay redirect payment by using the tester macro to call the should_make_worldline_giropay_redirect_payment method.
fn should_make_worldline_giropay_redirect_payment_test() {
    tester!(should_make_worldline_giropay_redirect_payment);
}

#[test]
#[serial]
/// This method is used to test whether the worldline ideal redirect payment should be made.
fn should_make_worldline_ideal_redirect_payment_test() {
    tester!(should_make_worldline_ideal_redirect_payment);
}

#[test]
#[serial]
/// This method is a test for the should_make_card_non_3ds_payment function. It uses the tester! macro to run the test case for making a non-3DS payment with a card. 
fn should_make_card_non_3ds_payment_test() {
    tester!(should_make_card_non_3ds_payment);
}
