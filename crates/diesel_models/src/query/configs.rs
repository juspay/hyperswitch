use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    configs::{Config, ConfigNew, ConfigUpdate, ConfigUpdateInternal},
    errors,
    schema::configs::dsl,
    PgPooledConn, StorageResult,
};

impl ConfigNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided PostgreSQL connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `Config` if successful, otherwise an error
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Config> {
        generics::generic_insert(conn, self).await
    }
}

impl Config {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by a given key.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `key` - A reference to the key used to search for the record
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    pub async fn find_by_key(conn: &PgPooledConn, key: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, key.to_owned()).await
    }

    #[instrument(skip(conn))]
        /// Asynchronously updates a record in the database based on the given key and configuration update. 
    /// If the record does not exist, it attempts to find the record by the key. 
    /// If no fields are found to update, it returns the existing record. 
    /// Returns a StorageResult containing the updated or found record, or an error if the update process fails.
    pub async fn update_by_key(
        conn: &PgPooledConn,
        key: &str,
        config_update: ConfigUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            key.to_owned(),
            ConfigUpdateInternal::from(config_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        key.to_owned(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously deletes a record from the database table by the specified key.
    ///
    /// # Arguments
    ///
    /// * `conn` - The PostgreSQL pooled connection
    /// * `key` - The key to use for deletion
    ///
    /// # Returns
    ///
    /// A `StorageResult` indicating whether the deletion was successful or not.
    pub async fn delete_by_key(conn: &PgPooledConn, key: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::key.eq(key.to_owned()))
            .await
    }
}
