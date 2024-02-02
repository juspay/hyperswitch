use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    merchant_key_store::{MerchantKeyStore, MerchantKeyStoreNew},
    schema::merchant_key_store::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantKeyStoreNew {
    #[instrument(skip(conn))]
        /// Inserts a new record into the merchant key store table in the database using the provided database connection.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    ///
    /// # Returns
    ///
    /// The result of the insertion operation, wrapped in a `StorageResult` enum
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantKeyStore> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantKeyStore {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection
    /// * `merchant_id` - The merchant ID to search for
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the result of the operation
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously deletes a record from the database based on the given merchant ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A pooled PostgreSQL connection
    /// * `merchant_id` - A reference to a string representing the merchant ID
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` indicating whether the deletion was successful
    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously retrieves a list of key stores that match the provided merchant IDs from the database.
    pub async fn list_multiple_key_stores(
        conn: &PgPooledConn,
        merchant_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as diesel::Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq_any(merchant_ids),
            None,
            None,
            None,
        )
        .await
    }
}
