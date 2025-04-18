use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;
use common_utils::id_type::{GlobalTokenId, MerchantId};
use serde::{Deserialize, Serialize};
use crate::{
    enums::{TokenizationFlag, TokenizationType},
    schema_v2::tokenization,
    PgPooledConn, StorageResult,
    query::generics,
};
use common_utils::{
    consts::MAX_LOCKER_ID_LENGTH,
};
use common_enums::ApiVersion;

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = tokenization)]
pub struct Tokenization {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub version: ApiVersion,
    pub flag: TokenizationFlag,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
pub struct TokenizationNew {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub version: ApiVersion,
    pub flag: TokenizationFlag,
}

impl TokenizationNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Tokenization> {
        generics::generic_insert(conn, self).await
    }
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = tokenization)]
pub struct TokenizationUpdate {
    pub updated_at: Option<PrimitiveDateTime>,
    pub version: Option<ApiVersion>,
    pub flag: Option<TokenizationFlag>,
}