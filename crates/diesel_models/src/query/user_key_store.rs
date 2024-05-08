use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    schema::user_key_store::dsl,
    user_key_store::{UserKeyStore, UserKeyStoreNew},
    PgPooledConn, StorageResult,
};

impl UserKeyStoreNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UserKeyStore> {
        generics::generic_insert(conn, self).await
    }
}

impl UserKeyStore {
    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }
}
