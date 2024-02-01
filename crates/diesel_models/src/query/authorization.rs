use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    authorization::{
        Authorization, AuthorizationNew, AuthorizationUpdate, AuthorizationUpdateInternal,
    },
    errors,
    schema::incremental_authorization::dsl,
    PgPooledConn, StorageResult,
};

impl AuthorizationNew {
    #[instrument(skip(conn))]
        /// This method takes a connection to a PostgreSQL database and inserts the Authorization object into the database using the generics::generic_insert function. It returns a StorageResult containing the inserted Authorization object.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Authorization> {
        generics::generic_insert(conn, self).await
    }
}

impl Authorization {
    #[instrument(skip(conn))]
        /// Updates an authorization record in the database based on the merchant ID and authorization ID,
    /// using the provided `AuthorizationUpdate` struct. If the authorization record does not exist, it
    /// returns an error. If there are no fields to update, it retrieves the existing authorization record
    /// and returns it. Returns a `StorageResult` containing the updated or retrieved authorization record.
    pub async fn update_by_merchant_id_authorization_id(
        conn: &PgPooledConn,
        merchant_id: String,
        authorization_id: String,
        authorization_update: AuthorizationUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::authorization_id.eq(authorization_id.to_owned())),
            AuthorizationUpdateInternal::from(authorization_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => Err(error.attach_printable(
                    "Authorization with the given Authorization ID does not exist",
                )),
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::merchant_id
                            .eq(merchant_id.to_owned())
                            .and(dsl::authorization_id.eq(authorization_id.to_owned())),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a list of items by matching the given merchant_id and payment_id in the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection
    /// * `merchant_id` - The merchant id to filter by
    /// * `payment_id` - The payment id to filter by
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing a vector of items matching the given merchant_id and payment_id
    /// 
    pub async fn find_by_merchant_id_payment_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
