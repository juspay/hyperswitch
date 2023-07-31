use serial_test::serial;
use test_utils::{selenium::*, tester};
use thirtyfour::{prelude::*, WebDriver};

struct WorldlineSeleniumTest;

impl SeleniumTest for WorldlineSeleniumTest {
    fn get_connector_name(&self) -> String {
        "worldline".to_string()
    }
}

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
fn should_make_worldline_giropay_redirect_payment_test() {
    tester!(should_make_worldline_giropay_redirect_payment);
}

#[test]
#[serial]
fn should_make_worldline_ideal_redirect_payment_test() {
    tester!(should_make_worldline_ideal_redirect_payment);
}

#[test]
#[serial]
fn should_make_card_non_3ds_payment_test() {
    tester!(should_make_card_non_3ds_payment);
}
