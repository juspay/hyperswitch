use common_utils::types::MinorUnit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayRequest {
    /// The identifier that is associated to a resource at the connector reference to which the relay request is being made
    pub connector_resource_id: String,
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The type of relay request
    #[serde(rename = "type")]
    pub relay_type: api_enums::RelayType,
    /// The data that is associated with the relay request
    pub data: Option<RelayData>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RelayData {
    /// The data that is associated with a refund relay request
    Refund(RelayRefundRequest),
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayRefundRequest {
    /// The amount that is being refunded
    pub amount: MinorUnit,
    /// The currency in which the amount is being refunded
    #[schema(value_type = Currency)]
    pub currency: api_enums::Currency,
    /// The reason for the refund
    pub reason: Option<String>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayResponse {
    /// The unique identifier for the Relay
    #[schema(value_type = String)]
    pub id: common_utils::id_type::RelayId,
    /// The status of the relay request
    #[schema(value_type = RelayStatus)]
    pub status: api_enums::RelayStatus,
    /// The error details if the relay request failed
    pub error: Option<RelayError>,
    /// The identifier that is associated to a resource at the connector reference to which the relay request is being made
    pub connector_resource_id: String,
    /// The identifier that is associated to a resource at the connector to which the relay request is being made
    pub connector_reference_id: Option<String>,
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    #[schema(value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The business profile that is associated with this relay request.
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    /// The type of relay request
    #[serde(rename = "type")]
    #[schema(value_type = RelayType)]
    pub relay_type: api_enums::RelayType,
    /// The data that is associated with the relay request
    pub data: Option<RelayData>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayError {
    /// The error code
    pub code: String,
    /// The error message
    pub message: String,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayRetrieveRequest {
    /// The unique identifier for the Relay
    #[serde(default)]
    pub force_sync: bool,
    /// The unique identifier for the Relay
    pub id: String,
}

impl common_utils::events::ApiEventMetric for RelayRequest {}

impl common_utils::events::ApiEventMetric for RelayResponse {}
