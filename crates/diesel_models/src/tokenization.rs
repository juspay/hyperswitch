#[cfg(feature = "v2")]
use common_enums;
#[cfg(feature = "v2")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
#[cfg(feature = "v2")]
use time::PrimitiveDateTime;

#[cfg(feature = "v2")]
use crate::schema_v2::tokenization;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = tokenization)]
pub struct Tokenization {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub locker_id: String,
    pub flag: common_enums::enums::TokenizationFlag,
    pub version: common_enums::enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
pub struct TokenizationNew {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub version: common_enums::enums::ApiVersion,
    pub flag: common_enums::enums::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = tokenization)]
pub struct TokenizationUpdate {
    pub updated_at: Option<PrimitiveDateTime>,
    pub version: Option<common_enums::enums::ApiVersion>,
    pub flag: Option<common_enums::enums::TokenizationFlag>,
}
