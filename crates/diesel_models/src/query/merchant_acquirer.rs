use diesel::{associations::HasTable, ExpressionMethods, Table};

use super::generics;
use crate::{
    errors::DatabaseError,
    merchant_acquirer::{MerchantAcquirer, MerchantAcquirerNew, MerchantAcquirerUpdate},
    schema::merchant_acquirer::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantAcquirerNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantAcquirer> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantAcquirer {
    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn find_by_id(
        conn: &PgPooledConn,
        merchant_acquirer_id: &common_utils::id_type::MerchantAcquirerId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_acquirer_id.eq(merchant_acquirer_id.to_owned()),
        )
        .await
    }

    pub async fn update_by_merchant_acquirer_id(
        conn: &PgPooledConn,
        merchant_acquirer_id: &common_utils::id_type::MerchantAcquirerId,
        merchant_acquirer_update: MerchantAcquirerUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            merchant_acquirer_id.to_owned(),
            merchant_acquirer_update,
        )
        .await
        {
            Err(error) => match error.current_context() {
                DatabaseError::NotFound => {
                    Err(error.attach_printable("Merchant Acquirer with the given ID doesn't exist"))
                }
                DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        merchant_acquirer_id.clone(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }
}
