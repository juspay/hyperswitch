use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    callback_mapper::CallbackMapper, schema::callback_mapper::dsl, PgPooledConn, StorageResult,
};

impl CallbackMapper {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

    pub async fn find_by_id(conn: &PgPooledConn, id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }
}
