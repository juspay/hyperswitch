use router::types;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct MollieTest;
impl ConnectorActions for MollieTest {}
impl utils::Connector for MollieTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Mollie;
        types::api::ConnectorData {
            connector: Box::new(&Mollie),
            connector_name: types::Connector::Mollie,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .mollie
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "mollie".to_string()
    }
}
