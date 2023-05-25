use serial_test::serial;
use thirtyfour::{prelude::*, WebDriver};

use crate::{selenium::*, tester};

struct AuthorizeDotNetSeleniumTest;

impl SeleniumTest for AuthorizeDotNetSeleniumTest {
    fn get_connector_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}

async fn should_make_authorizedotnet_3ds_mandate_payment(c: WebDriver) -> Result<(), WebDriverError> {
    let conn = AuthorizeDotNetSeleniumTest {};
    conn.make_redirection_payment(c, vec![
            Event::Trigger(Trigger::Goto(&format!("{CHEKOUT_BASE_URL}/card?cname=CL-BRW1&ccnum=4000027891380961&expmonth=10&expyear=25&cvv=123&amount=200&country=US&currency=USD&setup_future_usage=off_session&mandate_data[customer_acceptance][acceptance_type]=offline&mandate_data[customer_acceptance][accepted_at]=1963-05-03T04:07:52.723Z&mandate_data[customer_acceptance][online][ip_address]=in%20sit&mandate_data[customer_acceptance][online][user_agent]=amet%20irure%20esse&mandate_data[mandate_type][multi_use][amount]=7000&mandate_data[mandate_type][multi_use][currency]=USD&mandate_data[mandate_type][multi_use][start_date]=2022-09-10T00:00:00Z&mandate_data[mandate_type][multi_use][end_date]=2023-09-10T00:00:00Z&mandate_data[mandate_type][multi_use][metadata][frequency]=13&return_url={CHEKOUT_BASE_URL}/payments"))),
            Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            Event::Trigger(Trigger::Query(By::ClassName("title"))),
            Event::Assert(Assert::Eq(Selector::Title, "ThreeDS ACS Emulator - Challenge Page")),
            Event::Trigger(Trigger::Click(By::Id("btn1"))),
            Event::Trigger(Trigger::Click(By::Id("btn5"))),
            Event::Assert(Assert::IsPresent("succeeded")),
            Event::Assert(Assert::IsPresent("man_")),//mandate id prefix is present

    ]).await?;
    Ok(())
}