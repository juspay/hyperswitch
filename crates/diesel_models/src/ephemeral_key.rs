#[cfg(feature = "v2")]
use masking::{PeekInterface, Secret};
#[cfg(feature = "v2")]
pub struct EphemeralKeyTypeNew {
    pub id: common_utils::id_type::EphemeralKeyId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub secret: Secret<String>,
    pub resource_type: ResourceType,
}

#[cfg(feature = "v2")]
impl EphemeralKeyTypeNew {
    pub fn generate_secret_key(&self) -> String {
        format!("epkey_{}", self.secret.peek())
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EphemeralKeyType {
    pub id: common_utils::id_type::EphemeralKeyId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub resource_type: ResourceType,
    pub created_at: time::PrimitiveDateTime,
    pub expires: time::PrimitiveDateTime,
    pub secret: Secret<String>,
}

#[cfg(feature = "v2")]
impl EphemeralKeyType {
    pub fn generate_secret_key(&self) -> String {
        format!("epkey_{}", self.secret.peek())
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

#[derive(
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    PartialEq,
    Eq,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResourceType {
    Payment,
    PaymentMethod,
}
