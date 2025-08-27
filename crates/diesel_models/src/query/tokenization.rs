#[cfg(feature = "v2")]
use diesel::associations::HasTable;
#[cfg(feature = "v2")]
use diesel::ExpressionMethods;

#[cfg(feature = "v2")]
use crate::{
    errors, query::generics, schema_v2::tokenization, tokenization as tokenization_diesel,
    PgPooledConn, StorageResult,
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
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            tokenization::dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn update_with_id(
        self,
        conn: &PgPooledConn,
        tokenization_record: tokenization_diesel::TokenizationUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            tokenization::dsl::id.eq(self.id.to_owned()),
            tokenization_record,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }
}
