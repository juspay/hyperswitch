#[cfg(feature = "v2")]
use common_enums;
#[cfg(feature = "v2")]
use common_utils::{
    id_type::{GlobalTokenId, MerchantId},
    tokenization as tokenization_utils,
};
#[cfg(feature = "v2")]
use diesel::{
    associations::HasTable,
    deserialize::FromSqlRow,
    expression::AsExpression,
    pg::Pg,
    serialize::{Output, ToSql},
    sql_types::{Jsonb, Text},
    AsChangeset, Identifiable, Insertable, Queryable, Selectable,
};
#[cfg(feature = "v2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v2")]
use time::PrimitiveDateTime;

#[cfg(feature = "v2")]
use crate::{query::generics, schema_v2::tokenization, PgPooledConn, StorageResult};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = tokenization)]
pub struct Tokenization {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub locker_id: String,
    pub flag: common_enums::enums::TokenizationFlag,
    pub version: common_enums::enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl Tokenization {
    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::GlobalTokenId,
    ) -> StorageResult<Self> {
        use diesel::ExpressionMethods;
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            tokenization::dsl::id.eq(id.to_owned()),
        )
        .await
    }
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
pub struct TokenizationNew {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub version: common_enums::enums::ApiVersion,
    pub flag: common_enums::enums::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl Tokenization {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = tokenization)]
pub struct TokenizationUpdate {
    pub updated_at: Option<PrimitiveDateTime>,
    pub version: Option<common_enums::enums::ApiVersion>,
    pub flag: Option<common_enums::enums::TokenizationFlag>,
}
