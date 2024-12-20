pub use common_utils::types::MinorUnit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums;

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayRequest {
    /// The identifier that is associated to a resource at the connector to which the relay request is being made
    #[schema(example = "7256228702616471803954")]
    pub connector_resource_id: String,
    /// Identifier of the connector ( merchant connector account ) to which relay request is being made
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The type of relay request
    #[serde(rename = "type")]
    pub relay_type: RelayType,
    /// The data that is associated with the relay request
    pub data: Option<RelayData>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayType {
    /// The relay request is for a refund
    Refund,
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
    #[schema(value_type = i64 , example = 6540)]
    pub amount: MinorUnit,
    /// The currency in which the amount is being refunded
    #[schema(value_type = Currency)]
    pub currency: enums::Currency,
    /// The reason for the refund
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RelayResponse {
    /// The unique identifier for the Relay
    #[schema(example = "relay_mbabizu24mvu3mela5njyhpit4", value_type = String)]
    pub id: common_utils::id_type::RelayId,
    /// The status of the relay request
    pub status: RelayStatus,
    /// The reference identifier provided by the connector for the relay request
    #[schema(example = "pi_3MKEivSFNglxLpam0ZaL98q9")]
    pub connector_reference_id: Option<String>,
    /// The error details if the relay request failed
    pub error: Option<RelayError>,
    /// The identifier that is associated to a resource at the connector to which the relay request is being made
    #[schema(example = "7256228702616471803954")]
    pub connector_resource_id: String,
    /// Identifier of the connector ( merchant connector account ) to which relay request is being made
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The business profile that is associated with this relay request
    #[schema(example = "pro_abcdefghijklmnopqrstuvwxyz", value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    /// The type of relay request
    #[serde(rename = "type")]
    pub relay_type: RelayType,
    /// The data that is associated with the relay request
    pub data: Option<RelayData>,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayStatus {
    /// The relay request is successful
    Success,
    /// The relay request is being processed
    Processing,
    /// The relay request has failed
    Failure,
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
