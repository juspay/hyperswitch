use common_utils::pii;
use serde::{Deserialize, Serialize};

use crate::enums as api_enums;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyConnectorRequest {
    pub connector_name: api_enums::Connector,
    pub connector_account_details: Option<pii::SecretSerdeValue>,
}
