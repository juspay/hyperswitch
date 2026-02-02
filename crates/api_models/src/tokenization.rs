use common_utils::id_type;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::{schema, ToSchema};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenericTokenizationResponse {
    /// Unique identifier returned by the tokenization service
    #[schema(value_type = String, example = "12345_tok_01926c58bc6e77c09e809964e72af8c8")]
    pub id: id_type::GlobalTokenId,
    /// Created time of the tokenization id
    #[schema(value_type = PrimitiveDateTime,example = "2024-02-24T11:04:09.922Z")]
    pub created_at: PrimitiveDateTime,
    /// Status of the tokenization id created
    #[schema(value_type = String,example = "enabled")]
    pub flag: common_enums::TokenizationFlag,
}
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenericTokenizationRequest {
    /// Customer ID for which the tokenization is requested
    #[schema(value_type = String, example = "12345_cus_01926c58bc6e77c09e809964e72af8c8")]
    pub customer_id: id_type::GlobalCustomerId,
    /// Request for tokenization which contains the data to be tokenized
    #[schema(value_type = Object,example = json!({ "city": "NY", "unit": "245" }))]
    pub token_request: masking::Secret<serde_json::Value>,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteTokenDataRequest {
    /// Customer ID for which the tokenization is requested
    #[schema(value_type = String, example = "12345_cus_01926c58bc6e77c09e809964e72af8c8")]
    pub customer_id: id_type::GlobalCustomerId,
    /// Session ID associated with the tokenization request
    #[schema(value_type = String, example = "12345_pms_01926c58bc6e77c09e809964e72af8c8")]
    pub session_id: id_type::GlobalPaymentMethodSessionId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteTokenDataResponse {
    /// Unique identifier returned by the tokenization service
    #[schema(value_type = String, example = "12345_tok_01926c58bc6e77c09e809964e72af8c8")]
    pub id: id_type::GlobalTokenId,
}
