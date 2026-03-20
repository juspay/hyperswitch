use api_models::payment_methods::{CardDetailFromLocker, NetworkTokenResponse};
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type, pii};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::client::create::CardDetail;
#[derive(Clone, Debug)]
pub struct ModularListCustomerPaymentMethodsRequest;

/// Dummy modular service response payload.
#[derive(Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct ModularListCustomerPaymentMethodsResponse {
    pub customer_payment_methods: Vec<PaymentMethodResponseItem>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentMethodResponseItem {
    pub id: String,
    pub customer_id: id_type::CustomerId,
    pub payment_method_type: PaymentMethod,
    pub payment_method_subtype: PaymentMethodType,
    pub recurring_enabled: Option<bool>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub bank: Option<api_models::payment_methods::MaskedBankDetails>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub requires_cvv: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_used_at: PrimitiveDateTime,
    pub is_default: bool,
    pub billing: Option<api_models::payments::Address>,
    pub network_tokenization: Option<NetworkTokenResponse>,
    pub psp_tokenization_enabled: bool,
}
/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLocker),
}

/// V2 modular service request payload.
#[derive(Clone, Debug)]
pub struct ModularPMRetrieveRequest;

/// V2 PaymentMethodResponse as returned by the V2 API.
/// This is a copy of the V2 PaymentMethodResponse struct from api_models for use in V1-only builds.
#[derive(Clone, Debug, Deserialize)]
pub struct ModularPMRetrieveResponse {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_type: PaymentMethod,
    pub payment_method_subtype: PaymentMethodType,
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
    pub raw_payment_method_data: Option<RawPaymentMethodData>,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
    pub network_transaction_id: Option<String>,
}
/// V2 RawPaymentMethodData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawPaymentMethodData {
    Card(CardDetail),
}

/// V2 ConnectorTokenDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize, Serialize)]
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

/// V2 CardCVCTokenStorageDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,

    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
}
