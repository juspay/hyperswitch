use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::{admin, enums};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct VerifyConnectorRequest {
    pub connector_name: enums::Connector,
    pub connector_account_details: admin::ConnectorAuthType,
}

common_utils::impl_misc_api_event_type!(VerifyConnectorRequest);
