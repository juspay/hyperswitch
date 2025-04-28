pub use crate::cards::{DataDuplicationCheck, DeleteCardResp};
use api_models::{enums as api_enums, payment_methods::Card};
use cards::CardNumber;
use common_utils::id_type;
use masking::Secret;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum StoreLockerReq {
    LockerCard(StoreCardReq),
    LockerGeneric(StoreGenericReq),
}

impl StoreLockerReq {
    pub fn update_requestor_card_reference(&mut self, card_reference: Option<String>) {
        match self {
            Self::LockerCard(c) => c.requestor_card_reference = card_reference,
            Self::LockerGeneric(_) => (),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardReq {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestor_card_reference: Option<String>,
    pub card: Card,
    pub ttl: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreGenericReq {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    #[serde(rename = "enc_card_data")]
    pub enc_data: String,
    pub ttl: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub payload: Option<StoreCardRespPayload>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardRespPayload {
    pub card_reference: String,
    pub duplication_check: Option<DataDuplicationCheck>,
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
    pub card_security_code: Option<Secret<String>>,
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
