use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    batch_blocklist_job::{BatchBlocklistJob, BatchBlocklistJobNew, BatchBlocklistJobUpdate},
    schema::batch_blocklist_jobs::dsl,
    PgPooledConn, StorageResult,
};

impl BatchBlocklistJobNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BatchBlocklistJob> {
        generics::generic_insert(conn, self).await
    }
}

impl BatchBlocklistJob {
    pub async fn find_by_id_merchant_id(
        conn: &PgPooledConn,
        id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn count_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<usize> {
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    pub async fn update_by_id_merchant_id(
        conn: &PgPooledConn,
        id: &str,
        merchant_id: &str,
        update: BatchBlocklistJobUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            update,
        )
        .await
    }
}
