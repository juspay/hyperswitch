use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    pm_fingerprint::{PmFingerprint, PmFingerprintNew},
    schema::pm_fingerprint::dsl,
    PgPooledConn, StorageResult,
};

impl PmFingerprintNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PmFingerprint> {
        generics::generic_insert(conn, self).await
    }
}

impl PmFingerprint {
    #[instrument(skip(conn))]
    pub async fn find_by_fingerprint(
        conn: &PgPooledConn,
        fingerprint: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::fingerprint_id.eq(fingerprint),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_fingerprint(
        conn: &PgPooledConn,
        fingerprint: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::fingerprint_id.eq(fingerprint),
        )
        .await
    }
}
