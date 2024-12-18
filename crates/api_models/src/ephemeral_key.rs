use common_utils::id_type;
use serde;
use utoipa::ToSchema;

#[cfg(feature = "v1")]
/// Information required to create an ephemeral key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct EphemeralKeyCreateRequest {
    /// Customer ID for which an ephemeral key must be created
    #[schema(
        min_length = 1,
        max_length = 64,
        value_type = String,
        example = "cus_y3oqhf46pyzuxjbcn2giaqnb44"
    )]
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
/// Information required to create an ephemeral key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct EphemeralKeyCreateRequest {
    /// Customer ID for which an ephemeral key must be created
    #[schema(
        min_length = 32,
        max_length = 64,
        value_type = String,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8"
    )]
    pub customer_id: id_type::GlobalCustomerId,
}

impl common_utils::events::ApiEventMetric for EphemeralKeyCreateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

/// ephemeral_key for the customer_id mentioned
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq, ToSchema)]
pub struct EphemeralKeyCreateResponse {
    /// customer_id to which this ephemeral key belongs to
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
    /// time at which this ephemeral key was created
    pub created_at: i64,
    /// time at which this ephemeral key would expire
    pub expires: i64,
    /// ephemeral key
    pub secret: String,
}
