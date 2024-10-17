use router::types;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct MollieTest;
impl ConnectorActions for MollieTest {}
impl utils::Connector for MollieTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Mollie;
        utils::construct_connector_data_old(
            Box::new(Mollie::new()),
            types::Connector::Mollie,
            types::api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .mollie
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "mollie".to_string()
    }
}
