use diesel::associations::HasTable;

use crate::{cards_info::CardInfo, query::generics, PgPooledConn, StorageResult};

impl CardInfo {
    pub async fn find_by_iin(conn: &PgPooledConn, card_iin: &str) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            card_iin.to_owned(),
        )
        .await
    }
}
