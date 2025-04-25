use serde::{Deserialize, Serialize};
use common_utils::id_type::GlobalTokenId;
use time::PrimitiveDateTime;
use common_enums;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    pub id: GlobalTokenId,
    pub created_at: PrimitiveDateTime,
    pub flag: common_enums::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TokenizationFlag {
    Enabled,
    Disabled,
} 

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TokenizationQueryParameters{
    // Make the 
    pub reveal : Option<bool>,
}