use diesel::{associations::HasTable,ExpressionMethods};
use error_stack::report;

use crate::{cards_info::{CardInfo,UpdateCardInfo}, schema::cards_info::dsl, query::generics, PgPooledConn, StorageResult,errors};

impl CardInfo {
    pub async fn find_by_iin(conn: &PgPooledConn, card_iin: &str) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            card_iin.to_owned(),
        )
        .await
    }
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }
    pub async fn update(
        conn: &PgPooledConn,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            UpdateCardInfo,
            _,
            _,
        >(
            conn,
            dsl::card_iin
                .eq(card_iin),
                data,
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating card_info entry")
        })
    }
}
