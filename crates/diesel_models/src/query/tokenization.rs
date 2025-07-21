#[cfg(feature = "v2")]
use diesel::associations::HasTable;

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
