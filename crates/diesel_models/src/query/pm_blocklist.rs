use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    pm_blocklist::{PmBlocklist, PmBlocklistNew},
    schema::pm_blocklist::dsl,
    PgPooledConn, StorageResult,
};

impl PmBlocklistNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PmBlocklist> {
        generics::generic_insert(conn, self).await
    }
}

impl PmBlocklist {
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_hash(
        conn: &PgPooledConn,
        merchant_id: String,
        hash: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::pm_hash.eq(hash.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: String,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _,>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::id.asc()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_merchant_id_hash(
        conn: &PgPooledConn,
        merchant_id: String,
        hash: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::pm_hash.eq(hash.to_owned())),
        )
        .await
    }
}
