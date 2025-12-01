use std::fmt::Debug;

use api_models::enums as api_enums;
#[cfg(feature = "v1")]
use cards::CardNumber;
#[cfg(feature = "v2")]
use cards::{CardNumber, NetworkToken};
#[cfg(feature = "v2")]
use common_types::primitive_wrappers;
#[cfg(feature = "v2")]
use common_utils::generate_id;
use common_utils::id_type;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetails;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::api;
#[cfg(feature = "v2")]
use crate::{
    consts,
    types::{domain, storage},
};

#[cfg(feature = "v2")]
pub trait VaultingInterface {
    fn get_vaulting_request_url() -> &'static str;

    fn get_vaulting_flow_name() -> &'static str;
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintRequest {
    pub data: String,
    pub key: String,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintResponse {
    pub fingerprint_id: String,
}

#[cfg(any(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultRequest<D> {
    pub entity_id: id_type::GlobalCustomerId,
    pub vault_id: domain::VaultId,
    pub data: D,
    pub ttl: i64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultResponse {
    #[cfg(feature = "v2")]
    pub entity_id: Option<id_type::GlobalCustomerId>,
    #[cfg(feature = "v1")]
    pub entity_id: Option<id_type::CustomerId>,
    #[cfg(feature = "v2")]
    pub vault_id: domain::VaultId,
    #[cfg(feature = "v1")]
    pub vault_id: hyperswitch_domain_models::router_response_types::VaultIdType,
    pub fingerprint_id: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVault;

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetVaultFingerprint;

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieve;

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDelete;

#[cfg(feature = "v2")]
impl VaultingInterface for AddVault {
    fn get_vaulting_request_url() -> &'static str {
        consts::ADD_VAULT_REQUEST_URL
    }

    fn get_vaulting_flow_name() -> &'static str {
        consts::VAULT_ADD_FLOW_TYPE
    }
}

#[cfg(feature = "v2")]
impl VaultingInterface for GetVaultFingerprint {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_FINGERPRINT_REQUEST_URL
    }

    fn get_vaulting_flow_name() -> &'static str {
        consts::VAULT_GET_FINGERPRINT_FLOW_TYPE
    }
}

#[cfg(feature = "v2")]
impl VaultingInterface for VaultRetrieve {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_RETRIEVE_REQUEST_URL
    }

    fn get_vaulting_flow_name() -> &'static str {
        consts::VAULT_RETRIEVE_FLOW_TYPE
    }
}

#[cfg(feature = "v2")]
impl VaultingInterface for VaultDelete {
    fn get_vaulting_request_url() -> &'static str {
        consts::VAULT_DELETE_REQUEST_URL
    }

    fn get_vaulting_flow_name() -> &'static str {
        consts::VAULT_DELETE_FLOW_TYPE
    }
}

#[cfg(feature = "v2")]
pub struct SavedPMLPaymentsInfo {
    pub payment_intent: storage::PaymentIntent,
    pub profile: domain::Profile,
    pub collect_cvv_during_payment: Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub off_session_payment_flag: bool,
    pub is_connector_agnostic_mit_enabled: bool,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieveRequest {
    pub entity_id: id_type::GlobalCustomerId,
    pub vault_id: domain::VaultId,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultRetrieveResponse {
    pub data: domain::PaymentMethodVaultingData,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDeleteRequest {
    pub entity_id: id_type::GlobalCustomerId,
    pub vault_id: domain::VaultId,
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultDeleteResponse {
    pub entity_id: id_type::GlobalCustomerId,
    pub vault_id: domain::VaultId,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    pub card_security_code: Option<Secret<String>>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_security_code: Option<Secret<String>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    pub consent_id: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    pub consent_id: String,
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiPayload {
    pub service: String,
    pub card_data: Secret<String>, //encrypted card data
    pub order_data: OrderData,
    pub should_send_token: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CardNetworkTokenResponse {
    pub payload: Secret<String>, //encrypted payload
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNetworkTokenResponsePayload {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: Option<Secret<String>>,
    pub card_reference: String,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token: CardNumber,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateNetworkTokenResponsePayload {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: Option<Secret<String>>,
    pub card_reference: String,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token: NetworkToken,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize)]
pub struct GetCardToken {
    pub card_reference: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize)]
pub struct GetCardToken {
    pub card_reference: String,
    pub customer_id: id_type::GlobalCustomerId,
}

#[cfg(feature = "v1")]
#[derive(Debug, Deserialize)]
pub struct AuthenticationDetails {
    pub cryptogram: Secret<String>,
    pub token: CardNumber, //network token
}

#[cfg(feature = "v2")]
#[derive(Debug, Deserialize)]
pub struct AuthenticationDetails {
    pub cryptogram: Secret<String>,
    pub token: NetworkToken, //network token
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetails {
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub authentication_details: AuthenticationDetails,
    pub network: api_enums::CardNetwork,
    pub token_details: TokenDetails,
    pub eci: Option<String>,
    pub card_type: Option<String>,
    pub issuer: Option<String>,
    pub nickname: Option<Secret<String>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeleteNetworkTokenStatus {
    Success,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorInfo {
    pub code: String,
    pub developer_message: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorResponse {
    pub error_message: Option<String>,
    pub error_info: NetworkTokenErrorInfo,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteNetworkTokenResponse {
    pub status: DeleteNetworkTokenStatus,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTokenStatus {
    pub card_reference: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenStatus {
    pub card_reference: String,
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TokenStatus {
    Active,
    Inactive,
    Suspended,
    Expired,
    Deleted,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenStatusResponsePayload {
    pub token_status: TokenStatus,
    pub token_expiry_month: Option<Secret<String>>,
    pub token_expiry_year: Option<Secret<String>>,
    pub card_last_four: Option<String>,
    pub card_expiry_month: Option<Secret<String>>,
    pub card_expiry_year: Option<Secret<String>>,
    pub token_last_four: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckTokenStatusResponse {
    pub payload: CheckTokenStatusResponsePayload,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkTokenRequestorData {
    pub card_reference: String,
    pub customer_id: String,
    pub expiry_year: Secret<String>,
    pub expiry_month: Secret<String>,
}

impl NetworkTokenRequestorData {
    pub fn is_update_required(
        &self,
        data_stored_in_vault: api::payment_methods::CardDetailFromLocker,
    ) -> bool {
        //if the expiry year and month in the vault are not the same as the ones in the requestor data,
        //then we need to update the vault data with the updated expiry year and month.
        !((data_stored_in_vault.expiry_year.unwrap_or_default() == self.expiry_year)
            && (data_stored_in_vault.expiry_month.unwrap_or_default() == self.expiry_month))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkTokenMetaDataUpdateBody {
    pub token: NetworkTokenRequestorData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PanMetadataUpdateBody {
    pub card: NetworkTokenRequestorData,
}
