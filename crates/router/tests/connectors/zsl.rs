use router::types::{self, storage::enums};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

struct ZslTest;
impl ConnectorActions for ZslTest {}
impl utils::Connector for ZslTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Zsl;
        types::api::ConnectorData {
            connector: Box::new(&Zsl),
            connector_name: types::Connector::DummyConnector1,
            // Added as Dummy connector as template code is added for future usage
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .zsl
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "zsl".to_string()
    }
}

static CONNECTOR: ZslTest = ZslTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}
