use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    merchant_connector_account::{
        MerchantConnectorAccount, MerchantConnectorAccountNew,
        MerchantConnectorAccountUpdateInternal,
    },
    schema::merchant_connector_account::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantConnectorAccountNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record of MerchantConnectorAccount into the database using the provided connection.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantConnectorAccount> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantConnectorAccount {
    #[instrument(skip(conn))]
        /// Asynchronously updates a record in the database with the provided `merchant_connector_account` using the `conn` connection. Returns a `StorageResult` with the updated record on success or an error on failure. If there are no fields to update, it returns the original record. 
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            merchant_connector_account,
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

        /// Deletes a record from the database by the given merchant id and merchant connector id.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection
    /// * `merchant_id` - The identifier of the merchant
    /// * `merchant_connector_id` - The identifier of the merchant connector
    ///
    /// # Returns
    ///
    /// A `StorageResult` indicating whether the delete operation was successful or not.
    /// 
    pub async fn delete_by_merchant_id_merchant_connector_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID and connector label.
    pub async fn find_by_merchant_id_connector(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_label: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_label.eq(connector_label.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the storage by the given profile ID and connector name.
    pub async fn find_by_profile_id_connector_name(
        conn: &PgPooledConn,
        profile_id: &str,
        connector_name: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::connector_name.eq(connector_name.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a list of records by the given merchant ID and connector name in the database.
    pub async fn find_by_merchant_id_connector_name(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_name: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_name.eq(connector_name.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID and merchant connector ID.
    pub async fn find_by_merchant_id_merchant_connector_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds all records belonging to a specific merchant based on the provided merchant ID.
    /// 
    /// If get_disabled is true, it returns all records regardless of their disabled status.
    /// If get_disabled is false, it returns only records that are not disabled.
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        get_disabled: bool,
    ) -> StorageResult<Vec<Self>> {
        if get_disabled {
            generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
                None,
                None,
                Some(dsl::created_at.asc()),
            )
            .await
        } else {
            generics::generic_filter::<
                <Self as HasTable>::Table,
                _,
                <<Self as HasTable>::Table as Table>::PrimaryKey,
                _,
            >(
                conn,
                dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(dsl::disabled.eq(false)),
                None,
                None,
                None,
            )
            .await
        }
    }
}
