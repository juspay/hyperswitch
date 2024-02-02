use router::types;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct MollieTest;
impl ConnectorActions for MollieTest {}
impl utils::Connector for MollieTest {
        /// This method returns a ConnectorData object containing information about a specific connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Mollie;
        types::api::ConnectorData {
            connector: Box::new(&Mollie),
            connector_name: types::Connector::Mollie,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It first creates a new instance of ConnectorAuthentication and then converts it to ConnectorAuthType using the to_connector_auth_type function from the utils module.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .mollie
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "mollie" as a String.
    fn get_name(&self) -> String {
        "mollie".to_string()
    }
}
