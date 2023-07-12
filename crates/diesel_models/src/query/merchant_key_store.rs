use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    merchant_key_store::{MerchantKeyStore, MerchantKeyStoreNew},
    schema::merchant_key_store::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantKeyStoreNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantKeyStore> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantKeyStore {
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }
}
