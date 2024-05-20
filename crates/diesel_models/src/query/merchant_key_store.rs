use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    merchant_key_store::{MerchantKeyStore, MerchantKeyStoreNew},
    schema::merchant_key_store::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantKeyStoreNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantKeyStore> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantKeyStore {
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

    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    pub async fn list_multiple_key_stores(
        conn: &PgPooledConn,
        merchant_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as diesel::Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq_any(merchant_ids),
            None,
            None,
            None,
        )
        .await
    }
}
