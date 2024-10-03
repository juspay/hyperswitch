#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::generate_id;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use masking::Secret;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{
    consts,
    types::{api, domain, storage},
};

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
pub trait VaultingInterface {
    fn get_vaulting_request_url() -> &'static str;
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultId(String);

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl VaultId {
    pub fn get_string_repr(&self) -> &String {
        &self.0
    }

    pub fn generate(id: String) -> Self {
        Self(id)
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintRequest {
    pub data: String,
    pub key: String,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintResponse {
    pub fingerprint_id: String,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultRequest<D> {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: VaultId,
    pub data: D,
    pub ttl: i64,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultResponse {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: VaultId,
    pub fingerprint_id: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVault;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetVaultFingerprint;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieve;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDelete;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for AddVault {
    fn get_vaulting_request_url() -> &'static str {
        consts::ADD_VAULT_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for GetVaultFingerprint {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_FINGERPRINT_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for VaultRetrieve {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_RETRIEVE_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for VaultDelete {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_RETRIEVE_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum PaymentMethodVaultingData {
    Card(api::CardDetail),
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingDataInterface for PaymentMethodVaultingData {
    fn get_vaulting_data_key(&self) -> String {
        match &self {
            Self::Card(card) => card.card_number.to_string(),
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub struct PaymentMethodClientSecret;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodClientSecret {
    pub fn generate(payment_method_id: &common_utils::id_type::GlobalPaymentMethodId) -> String {
        todo!()
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub struct SavedPMLPaymentsInfo {
    pub payment_intent: storage::PaymentIntent,
    pub business_profile: Option<domain::Profile>,
    pub requires_cvv: bool,
    pub off_session_payment_flag: bool,
    pub is_connector_agnostic_mit_enabled: bool,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieveRequest {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: VaultId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieveResponse {
    pub data: Secret<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDeleteRequest {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: VaultId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDeleteResponse {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: VaultId,
}
