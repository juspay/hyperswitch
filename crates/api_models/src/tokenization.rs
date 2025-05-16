use common_enums;
use common_utils::id_type::{GlobalCustomerId, GlobalTokenId};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::{schema, ToSchema};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenericTokenizationResponse {
    #[schema(value_type = String, example = "12345_tok_01926c58bc6e77c09e809964e72af8c8")]
    pub id: GlobalTokenId,
    #[schema(value_type = PrimitiveDateTime,example = "2024-02-24T11:04:09.922Z")]
    pub created_at: PrimitiveDateTime,
    #[schema(value_type = String,example = "enabled")]
    pub flag: common_enums::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TokenizationFlag {
    Enabled,
    Disabled,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenericTokenizationRequest {
    #[schema(value_type = String, example = "12345_cus_01926c58bc6e77c09e809964e72af8c8")]
    pub customer_id: GlobalCustomerId,
    #[schema(value_type = Object,example = json!({ "city": "NY", "unit": "245" }))]
    pub token_request: masking::Secret<serde_json::Value>,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TokenizationQueryParameters {
    pub reveal: Option<bool>,
}
