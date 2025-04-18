use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;
use common_utils::id_type::{GlobalTokenId, MerchantId};
use serde::{Deserialize, Serialize};
use common_utils::pii;
use validator::Validate;
use common_utils::consts::MAX_LOCKER_ID_LENGTH;

use crate::{enums as storage_enums, schema::tokenization};
use crate::types;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize, Validate)]
#[diesel(table_name = tokenization, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Tokenization {
    pub id: GlobalTokenId,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    #[validate(length(min = 1, max = "MAX_LOCKER_ID_LENGTH"))]
    pub locker_id: String,
    pub flag: types::TokenizationFlag,
    pub version: types::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable, Serialize, Deserialize, Validate)]
#[diesel(table_name = tokenization)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenizationNew {
    pub merchant_id: MerchantId,
    #[validate(length(min = 1, max = "MAX_LOCKER_ID_LENGTH"))]
    pub locker_id: String,
    pub flag: types::TokenizationFlag,
    pub version: types::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = tokenization)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenizationUpdate {
    pub status: Option<storage_enums::TokenizationStatus>,
    pub updated_at: PrimitiveDateTime,
} 