use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    dispute::{Dispute, DisputeNew, DisputeUpdate, DisputeUpdateInternal},
    errors,
    schema::dispute::dsl,
    PgPooledConn, StorageResult,
};

impl DisputeNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts the current Dispute object into the database using the provided PostgreSQL connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `Dispute` object if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Dispute> {
        generics::generic_insert(conn, self).await
    }
}

impl Dispute {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID, payment ID, and connector dispute ID. 
    /// 
    /// # Arguments
    /// * `conn` - The PostgreSQL database connection
    /// * `merchant_id` - The merchant ID to search for
    /// * `payment_id` - The payment ID to search for
    /// * `connector_dispute_id` - The connector dispute ID to search for
    /// 
    /// # Returns
    /// An `Option` containing the found record if it exists, or `None` if not.
    pub async fn find_by_merchant_id_payment_id_connector_dispute_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned()))
                .and(dsl::connector_dispute_id.eq(connector_dispute_id.to_owned())),
        )
        .await
    }

        /// Asynchronously finds a record in the database by the given merchant_id and dispute_id.
    pub async fn find_by_merchant_id_dispute_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        dispute_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::dispute_id.eq(dispute_id.to_owned())),
        )
        .await
    }

        /// Asynchronously finds a list of items by the given merchant ID and payment ID.
    pub async fn find_by_merchant_id_payment_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
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
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Updates the current `Dispute` instance in the database with the provided `DisputeUpdate`, using the given PostgreSQL connection.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `dispute` - The `DisputeUpdate` instance containing the updated values
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the updated `Dispute` instance if successful, or an error if the update fails.
    pub async fn update(self, conn: &PgPooledConn, dispute: DisputeUpdate) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::dispute_id.eq(self.dispute_id.to_owned()),
            DisputeUpdateInternal::from(dispute),
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
