use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    call_back_mapper::{CallBackMapper, CallBackMapperNew},
    schema::call_back_mapper::dsl,
    PgPooledConn, StorageResult,
};

impl CallBackMapperNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<CallBackMapper> {
        generics::generic_insert(conn, self).await
    }
}

impl CallBackMapper {
    pub async fn find_by_id(conn: &PgPooledConn, id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }
}
