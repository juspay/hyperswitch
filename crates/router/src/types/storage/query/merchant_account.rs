use diesel::{associations::HasTable, ExpressionMethods};
use router_env::tracing::{self, instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::merchant_account::dsl,
    types::storage::{
        MerchantAccount, MerchantAccountNew, MerchantAccountUpdate, MerchantAccountUpdateInternal,
    },
};

impl MerchantAccountNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        generics::generic_insert::<<MerchantAccount as HasTable>::Table, _, _>(conn, self).await
    }
}

impl MerchantAccount {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            MerchantAccountUpdateInternal::from(merchant_account),
        )
        .await
    }

    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_api_key(
        conn: &PgPooledConn,
        api_key: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::api_key.eq(api_key.to_owned()),
        )
        .await
    }

    #[instrument(skip_all)]
    pub async fn find_by_publishable_key(
        conn: &PgPooledConn,
        publishable_key: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::publishable_key.eq(publishable_key.to_owned()),
        )
        .await
    }
}
