use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    blocklist_lookup::{BlocklistLookup, BlocklistLookupNew},
    schema::blocklist_lookup::dsl,
    PgPooledConn, StorageResult,
};

impl BlocklistLookupNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BlocklistLookup> {
        generics::generic_insert(conn, self).await
    }
}

impl BlocklistLookup {
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_kms_encrypted_hash(
        conn: &PgPooledConn,
        merchant_id: String,
        kms_encrypted_hash: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::kms_decrypted_hash.eq(kms_encrypted_hash.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_merchant_id_kms_decrypted_hash(
        conn: &PgPooledConn,
        merchant_id: String,
        kms_decrypted_hash: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::kms_decrypted_hash.eq(kms_decrypted_hash.to_owned())),
        )
        .await
    }
}
