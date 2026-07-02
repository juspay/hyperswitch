use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    kv,
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    schema::reverse_lookup::dsl,
    PgPooledConn, StorageResult,
};

impl ReverseLookupNew {
    pub async fn insert(self, conn: &mut PgPooledConn) -> StorageResult<ReverseLookup> {
        generics::generic_insert(conn, self).await
    }

    pub async fn batch_insert(
        reverse_lookups: Vec<Self>,
        conn: &mut PgPooledConn,
    ) -> StorageResult<()> {
        generics::generic_insert::<_, _, ReverseLookup>(conn, reverse_lookups).await?;
        Ok(())
    }

    pub async fn generate_drainer_insert_query(
        self,
        conn: &mut PgPooledConn,
    ) -> StorageResult<kv::SerializableQuery> {
        kv::generate_insert_query(conn, self).await
    }
}
impl ReverseLookup {
    pub async fn find_by_lookup_id(lookup_id: &str, conn: &mut PgPooledConn) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::lookup_id.eq(lookup_id.to_owned()),
        )
        .await
    }
}
