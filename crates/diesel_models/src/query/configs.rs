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
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Config> {
        generics::generic_insert(conn, self).await
    }
}

impl Config {
    #[instrument(skip(conn))]
    pub async fn find_by_key(conn: &PgPooledConn, key: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, key.to_owned()).await
    }

    #[instrument(skip(conn))]
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
    pub async fn delete_by_key(conn: &PgPooledConn, key: &str) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::key.eq(key.to_owned()),
        )
        .await
    }
}
