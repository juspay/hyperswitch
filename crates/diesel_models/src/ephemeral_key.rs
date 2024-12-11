#[cfg(feature = "v2")]
pub struct EphemeralKeyNew {
    pub id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub secret: String,
    pub resource_type: ResourceType,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EphemeralKey {
    pub id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub resource_type: ResourceType,
    pub created_at: i64,
    pub expires: i64,
    pub secret: String,
}

#[cfg(feature = "v1")]
pub struct EphemeralKeyNew {
    pub id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub secret: String,
}

#[cfg(feature = "v1")]
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
