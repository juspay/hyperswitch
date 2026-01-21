use router::types::{self, api};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct CapitecvrpTest;
impl ConnectorActions for CapitecvrpTest {}
impl utils::Connector for CapitecvrpTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Capitecvrp;
        utils::construct_connector_data_old(
            Box::new(Capitecvrp::new()),
            types::Connector::Capitecvrp,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .capitecvrp
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "capitecvrp".to_string()
    }
}

static CONNECTOR: CapitecvrpTest = CapitecvrpTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

// Capitec VRP connector tests
// Note: Capitec VRP uses a consent-based flow:
// 1. Create consent (SetupMandate)
// 2. Client approves consent in Capitec app
// 3. Execute payment action with consent receipt

#[actix_web::test]
async fn should_create_consent() {
    // Test consent creation (SetupMandate flow)
    // This requires proper mandate data to be set up
    let _response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await;
    // Consent creation returns AuthenticationPending status
}
