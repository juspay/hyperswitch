use diesel::associations::HasTable;
use router_env::tracing::{self, instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    types::storage::{Config, ConfigNew, ConfigUpdate, ConfigUpdateInternal},
};

impl ConfigNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Config, errors::StorageError> {
        generics::generic_insert::<<Config as HasTable>::Table, _, _>(conn, self).await
    }
}

impl Config {
    #[instrument(skip(conn))]
    pub async fn find_by_key(
        conn: &PgPooledConn,
        key: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, key.to_owned()).await
    }

    #[instrument(skip(conn))]
    pub async fn update_by_key(
        conn: &PgPooledConn,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            key.to_owned(),
            ConfigUpdateInternal::from(config_update),
        )
        .await
    }
}
