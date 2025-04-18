use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;
use common_utils::id_type::{GlobalTokenId, MerchantId};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{enums as storage_enums, schema::tokenization};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = tokenization, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Tokenization {
    pub id: GlobalTokenId,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub token_data: Option<Secret<String>>,
    pub locker_id: String,
    pub status: storage_enums::TokenizationStatus,
    pub metadata: Option<Secret<String>>,
    pub version: storage_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenizationNew {
    pub merchant_id: MerchantId,
    pub token_data: Option<Secret<String>>,
    pub locker_id: String,
    pub status: storage_enums::TokenizationStatus,
    pub metadata: Option<Secret<String>>,
    pub version: storage_enums::ApiVersion,
} 