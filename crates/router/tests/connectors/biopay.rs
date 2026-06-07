use hyperswitch_masking::Secret;
use router::types::{self, api};

use crate::utils::{self, Connector as ConnectorTest, ConnectorActions};

struct BiopayTest;

impl ConnectorActions for BiopayTest {}

impl ConnectorTest for BiopayTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Biopay;

        utils::construct_connector_data_old(
            Box::new(Biopay::new()),
            types::Connector::Biopay,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::HeaderKey {
            api_key: Secret::new("test_biopay_platform_secret".to_string()),
        }
    }

    fn get_name(&self) -> String {
        "biopay".to_string()
    }
}

#[test]
fn should_construct_biopay_connector_data() {
    let connector = BiopayTest {};

    let _connector_data = connector.get_data();
    let _auth = connector.get_auth_token();

    assert_eq!(connector.get_name(), "biopay".to_string());
}
