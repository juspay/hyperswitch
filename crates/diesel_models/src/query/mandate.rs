use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use error_stack::report;
use router_env::{instrument, tracing};

use super::generics;
use crate::{errors, mandate::*, schema::mandate::dsl, PgPooledConn, StorageResult};

impl MandateNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided connection and returns the inserted record if successful.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Mandate> {
        generics::generic_insert(conn, self).await
    }
}

impl Mandate {
        /// Asynchronously finds a record in the database by the given merchant ID and mandate ID.
    pub async fn find_by_merchant_id_mandate_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        mandate_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::mandate_id.eq(mandate_id.to_owned())),
        )
        .await
    }

        /// Asynchronously finds a record in the database by the given merchant ID and connector mandate ID. 
    pub async fn find_by_merchant_id_connector_mandate_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_mandate_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_mandate_id.eq(connector_mandate_id.to_owned())),
        )
        .await
    }

        /// Asynchronously finds records by matching the merchant_id and customer_id in the database using the provided connection. 
    /// Returns a vector of the found records or an error if the operation fails.
    pub async fn find_by_merchant_id_customer_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        customer_id: &str,
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
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

        /// Updates a mandate in the database based on the given merchant ID and mandate ID, using the provided mandate update information. Returns a result containing the updated mandate or an error if the mandate is not found.
    pub async fn update_by_merchant_id_mandate_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        mandate_id: &str,
        mandate: MandateUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::mandate_id.eq(mandate_id.to_owned())),
            MandateUpdateInternal::from(mandate),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating mandate")
        })
    }
}
