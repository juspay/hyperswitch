#[cfg(feature = "v2")]
use masking::{PeekInterface, Secret};

#[cfg(feature = "v2")]
pub struct ClientSecretTypeNew {
    pub id: common_utils::id_type::ClientSecretId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub secret: Secret<String>,
    pub resource_id: common_utils::types::authentication::ResourceId,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientSecretType {
    pub id: common_utils::id_type::ClientSecretId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub resource_id: common_utils::types::authentication::ResourceId,
    pub created_at: time::PrimitiveDateTime,
    pub expires: time::PrimitiveDateTime,
    pub secret: Secret<String>,
}

#[cfg(feature = "v2")]
impl ClientSecretType {
    pub fn generate_secret_key(&self) -> String {
        format!("cs_{}", self.secret.peek())
    }
}

pub struct EphemeralKeyNew {
    pub id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub secret: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EphemeralKey {
    pub id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub created_at: i64,
    pub expires: i64,
    pub secret: String,
}

impl common_utils::events::ApiEventMetric for EphemeralKey {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
