use serde;
use utoipa::ToSchema;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq, ToSchema)]
pub struct EphemeralKeyCreateResponse {
    /// customer_id to which this ephemeral key belongs to
    pub customer_id: String,
    /// time at which this ephemeral key was created
    pub created_at: i64,
    /// time at which this ephemeral key would expire
    pub expires: i64,
    /// ephemeral key
    pub secret: String,
}
