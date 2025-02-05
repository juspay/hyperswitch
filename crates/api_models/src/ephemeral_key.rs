use common_utils::id_type;
#[cfg(feature = "v2")]
use masking::Secret;
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceId {
    #[schema(value_type = String)]
    Customer(id_type::GlobalCustomerId),
}

#[cfg(feature = "v2")]
/// Information required to create a client secret.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ClientSecretCreateRequest {
    /// Resource ID for which a client secret must be created
    pub resource_id: ResourceId,
}

#[cfg(feature = "v2")]
/// client_secret for the resource_id mentioned
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq, ToSchema)]
pub struct ClientSecretResponse {
    /// Client Secret id
    #[schema(value_type = String, max_length = 32, min_length = 1)]
    pub id: id_type::ClientSecretId,
    /// resource_id to which this client secret belongs to
    #[schema(value_type = ResourceId)]
    pub resource_id: ResourceId,
    /// time at which this client secret was created
    pub created_at: time::PrimitiveDateTime,
    /// time at which this client secret would expire
    pub expires: time::PrimitiveDateTime,
    #[schema(value_type=String)]
    /// client secret
    pub secret: Secret<String>,
}

#[cfg(feature = "v1")]
impl common_utils::events::ApiEventMetric for EphemeralKeyCreateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

#[cfg(feature = "v1")]
impl common_utils::events::ApiEventMetric for EphemeralKeyCreateResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

#[cfg(feature = "v2")]
impl common_utils::events::ApiEventMetric for ClientSecretCreateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

#[cfg(feature = "v2")]
impl common_utils::events::ApiEventMetric for ClientSecretResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

#[cfg(feature = "v1")]
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
