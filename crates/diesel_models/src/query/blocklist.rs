use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    blocklist::{Blocklist, BlocklistNew},
    schema::blocklist::dsl,
    PgPooledConn, StorageResult,
};

impl BlocklistNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Blocklist> {
        generics::generic_insert(conn, self).await
    }
}

impl Blocklist {
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_fingerprint_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn list_by_merchant_id_data_kind(
        conn: &PgPooledConn,
        merchant_id: &str,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::data_kind.eq(data_kind.to_owned())),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_merchant_id_fingerprint_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }
}
