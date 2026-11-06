#[cfg(feature = "v2")]
use hyperswitch_masking::{PeekInterface, Secret};

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
