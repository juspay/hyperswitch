use diesel::associations::HasTable;

use crate::{cards_info::CardInfo, query::generics, PgPooledConn, StorageResult};

impl CardInfo {
        /// Asynchronously finds a record in the database by the provided card issuer identification number (IIN).
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a `PgPooledConn` which represents a pooled connection to a PostgreSQL database.
    /// * `card_iin` - A reference to a `str` which represents the card issuer identification number to search for.
    /// 
    /// # Returns
    /// 
    /// An `Option` containing the found record if it exists, or `None` if no record with the specified card IIN is found.
    pub async fn find_by_iin(conn: &PgPooledConn, card_iin: &str) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            card_iin.to_owned(),
        )
        .await
    }
}
