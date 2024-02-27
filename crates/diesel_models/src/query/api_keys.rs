use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    api_keys::{ApiKey, ApiKeyNew, ApiKeyUpdate, ApiKeyUpdateInternal, HashedApiKey},
    errors,
    schema::api_keys::dsl,
    PgPooledConn, StorageResult,
};

impl ApiKeyNew {
    pub async fn insert_api_key(self, conn: &PgPooledConn) -> StorageResult<ApiKey> {
        generics::generic_insert(conn, self).await
    }
}

impl ApiKey {
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
