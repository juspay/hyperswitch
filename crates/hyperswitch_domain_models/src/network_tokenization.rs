use std::fmt::Debug;

use api_models::enums as api_enums;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use cards::CardNumber;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use cards::{CardNumber, NetworkToken};
use common_utils::id_type;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    pub card_security_code: Secret<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_security_code: Option<Secret<String>>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    pub consent_id: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
    pub key_id: String,
    pub should_send_token: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CardNetworkTokenResponse {
    pub payload: Secret<String>, //encrypted payload
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Serialize)]
pub struct GetCardToken {
    pub card_reference: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Serialize)]
pub struct GetCardToken {
    pub card_reference: String,
    pub customer_id: id_type::GlobalCustomerId,
}
#[derive(Debug, Deserialize)]
pub struct AuthenticationDetails {
    pub cryptogram: Secret<String>,
    pub token: CardNumber, //network token
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
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::CustomerId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
    pub error_message: String,
    pub error_info: NetworkTokenErrorInfo,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteNetworkTokenResponse {
    pub status: DeleteNetworkTokenStatus,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTokenStatus {
    pub card_reference: String,
    pub customer_id: id_type::CustomerId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTokenStatus {
    pub card_reference: String,
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TokenStatus {
    Active,
    Inactive,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenStatusResponsePayload {
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_status: TokenStatus,
}

#[derive(Debug, Deserialize)]
pub struct CheckTokenStatusResponse {
    pub payload: CheckTokenStatusResponsePayload,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub type NetworkTokenNumber = CardNumber;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub type NetworkTokenNumber = NetworkToken;
