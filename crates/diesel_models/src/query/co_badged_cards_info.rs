use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    co_badged_cards_info::CoBadgedCardInfo, errors, schema::co_badged_cards_info::dsl,
    PgPooledConn, StorageResult, UpdateCoBadgedCardInfo,
};

impl CoBadgedCardInfo {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

    pub async fn find_by_bin(conn: &PgPooledConn, card_number: i64) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::card_bin_min
                .le(card_number)
                .and(dsl::card_bin_max.ge(card_number)),
            None,
            None,
            Some(dsl::modified_at.asc()),
        )
        .await
    }

    pub async fn update(
        self,
        conn: &PgPooledConn,
        data: UpdateCoBadgedCardInfo,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::id.eq(self.id.clone()), data)
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
