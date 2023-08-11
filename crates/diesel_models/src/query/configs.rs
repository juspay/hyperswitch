use super::generics;
// use crate::query::generics::Insert;
use crate::{
    configs::{Config, ConfigNew, ConfigUpdate, ConfigUpdateInternal},
    errors,
    schema::{self, configs::dsl},
    PgPooledConn, StorageResult,
};
use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

impl ConfigNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Config> {
        let q = generics::generic_insert_query::<schema::configs::table, ConfigNew, Config>(self).on_conflict_do_nothing();
        generics::insert::<schema::configs::table, _, _>(q, conn).await
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
    pub async fn delete_by_key(conn: &PgPooledConn, key: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::key.eq(key.to_owned()))
            .await
    }
}
