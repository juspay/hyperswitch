use api_models::payment_methods::CardDetailFromLocker;
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type, pii};
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetailsPaymentMethod;
use serde::Deserialize;
use time::PrimitiveDateTime;

/// V2 modular service request payload.
#[derive(Clone, Debug)]
pub struct ModularPMRetrieveResquest;

/// V2 PaymentMethodResponse as returned by the V2 API.
/// This is a copy of the V2 PaymentMethodResponse struct from api_models for use in V1-only builds.
#[derive(Clone, Debug, Deserialize)]
pub struct ModularPMRetrieveResponse {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_type: Option<PaymentMethod>,
    pub payment_method_subtype: Option<PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

/// V2 ConnectorTokenDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorTokenDetails {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub token_type: common_enums::TokenizationType,
    pub status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<common_utils::types::MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub token: masking::Secret<String>,
}

/// V2 NetworkTokenResponse (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct NetworkTokenResponse {
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}

/// V2 CardCVCTokenStorageDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,

    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
}

/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLocker),
}
