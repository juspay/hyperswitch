use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    api_keys::{ApiKey, ApiKeyNew, ApiKeyUpdate, ApiKeyUpdateInternal, HashedApiKey},
    errors,
    schema::api_keys::dsl,
    PgPooledConn, StorageResult,
};

impl ApiKeyNew {
    #[instrument(skip(conn))]
        /// Inserts a new record into the database using the provided database connection.
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled database connection
    /// 
    /// # Returns
    /// The inserted `ApiKey` record wrapped in a `StorageResult`
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ApiKey> {
        generics::generic_insert(conn, self).await
    }
}

impl ApiKey {
    #[instrument(skip(conn))]
        /// Updates an API key identified by the merchant ID and key ID with the provided new data.
    /// If the API key with the given key ID does not exist, an error is returned. If there are no
    /// fields to update, the existing API key is returned. Otherwise, the API key is updated with
    /// the new data and the updated API key is returned.
    pub async fn update_by_merchant_id_key_id(
        conn: &PgPooledConn,
        merchant_id: String,
        key_id: String,
        api_key_update: ApiKeyUpdate,
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
                .and(dsl::key_id.eq(key_id.to_owned())),
            ApiKeyUpdateInternal::from(api_key_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    Err(error.attach_printable("API key with the given key ID does not exist"))
                }
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::merchant_id
                            .eq(merchant_id.to_owned())
                            .and(dsl::key_id.eq(key_id.to_owned())),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Revokes a key by its associated merchant ID and key ID from the database.
    pub async fn revoke_by_merchant_id_key_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        key_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::key_id.eq(key_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds an optional record by the given merchant ID and key ID in the database. 
    /// If a record matching the provided merchant ID and key ID is found, it returns Some(record), 
    /// otherwise it returns None.
    pub async fn find_optional_by_merchant_id_key_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        key_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::key_id.eq(key_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds an optional instance of a struct by the given hashed API key in the database.
    pub async fn find_optional_by_hashed_api_key(
        conn: &PgPooledConn,
        hashed_api_key: HashedApiKey,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::hashed_api_key.eq(hashed_api_key),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a list of items by their merchant ID in the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - The database connection
    /// * `merchant_id` - The ID of the merchant
    /// * `limit` - Optional limit for the number of items to retrieve
    /// * `offset` - Optional offset for the items to retrieve
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing a vector of items found in the database
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            limit,
            offset,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
