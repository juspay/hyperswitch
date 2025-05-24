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
use crate::{
    query::generics, schema_v2::tokenization, tokenization as tokenization_diesel, PgPooledConn,
    StorageResult,
};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl tokenization_diesel::Tokenization {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

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
