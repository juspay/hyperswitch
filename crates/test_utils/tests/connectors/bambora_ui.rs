use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct BamboraSeleniumTest;

impl SeleniumTest for BamboraSeleniumTest {
    fn get_connector_name(&self) -> String {
        "bambora".to_string()
    }
}

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
fn should_make_3ds_payment_test() {
    tester!(should_make_3ds_payment);
}
