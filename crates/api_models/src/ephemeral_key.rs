use common_utils::id_type;
use serde;
use utoipa::ToSchema;

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
