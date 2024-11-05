use router::types;
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct GpaymentsTest;
impl ConnectorActions for GpaymentsTest {}
impl utils::Connector for GpaymentsTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Gpayments;
        utils::construct_connector_data_old(
            Box::new(Gpayments::new()),
            types::Connector::Threedsecureio,
            // Added as Dummy connector as template code is added for future usage
            types::api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .gpayments
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "gpayments".to_string()
    }
}
