use common_utils::{
    crypto::Encryptable, encryption::Encryption, errors::CustomResult, pii,
    types::keymanager::ToEncryptable,
};
use masking::Secret;
use rustc_hash::FxHashMap;
use serde_json::Value;

#[cfg(feature = "v1")]
#[derive(Clone, Debug, router_derive::ToEncryption, serde::Serialize)]
pub struct Authentication {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub authentication_connector: Option<String>,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub ds_trans_id: Option<String>,
    pub directory_server_id: Option<String>,
    pub acquirer_country_code: Option<String>,
    pub service_details: Option<Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub authentication_client_secret: Option<String>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub return_url: Option<String>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    #[encrypt(ty = Value)]
    pub billing_address: Option<Encryptable<crate::address::Address>>,
    #[encrypt(ty = Value)]
    pub shipping_address: Option<Encryptable<crate::address::Address>>,
    pub browser_info: Option<Value>,
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PgRedirectResponseForAuthentication {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub status: common_enums::TransactionStatus,
    pub gateway_id: String,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub amount: Option<common_utils::types::MinorUnit>,
}
