use diesel::{associations::HasTable, ExpressionMethods};
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
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ApiKey> {
        generics::generic_insert(conn, self).await
    }
}

impl ApiKey {
    #[instrument(skip(conn))]
    pub async fn update_by_key_id(
        conn: &PgPooledConn,
        key_id: String,
        api_key_update: ApiKeyUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            key_id.clone(),
            ApiKeyUpdateInternal::from(api_key_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    Err(error.attach_printable("API key with the given key ID does not exist"))
                }
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, key_id)
                        .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
    pub async fn revoke_by_key_id(conn: &PgPooledConn, key_id: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::key_id.eq(key_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_key_id(
        conn: &PgPooledConn,
        key_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            key_id.to_owned(),
        )
        .await
    }

    #[instrument(skip(conn))]
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
