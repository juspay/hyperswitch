use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    merchant_connector_account::{
        MerchantConnectorAccount, MerchantConnectorAccountNew, MerchantConnectorAccountUpdate,
        MerchantConnectorAccountUpdateInternal,
    },
    schema::merchant_connector_account::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantConnectorAccountNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantConnectorAccount> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantConnectorAccount {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            MerchantConnectorAccountUpdateInternal::from(merchant_connector_account),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn delete_by_merchant_id_merchant_connector_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_connector(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_name.eq(connector.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_merchant_connector_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        get_disabled: bool,
    ) -> StorageResult<Vec<Self>> {
        if get_disabled {
            generics::generic_filter::<
                <Self as HasTable>::Table,
                _,
                <<Self as HasTable>::Table as Table>::PrimaryKey,
                _,
            >(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
                None,
                None,
                None,
            )
            .await
        } else {
            generics::generic_filter::<
                <Self as HasTable>::Table,
                _,
                <<Self as HasTable>::Table as Table>::PrimaryKey,
                _,
            >(
                conn,
                dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(dsl::disabled.eq(false)),
                None,
                None,
                None,
            )
            .await
        }
    }
}
