use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    blocklist_fingerprint::{BlocklistFingerprint, BlocklistFingerprintNew},
    schema::blocklist_fingerprint::dsl,
    PgPooledConn, StorageResult,
};

impl BlocklistFingerprintNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BlocklistFingerprint> {
        generics::generic_insert(conn, self).await
    }
}

impl BlocklistFingerprint {
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
}
