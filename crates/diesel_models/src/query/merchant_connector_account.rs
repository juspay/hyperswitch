use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::merchant_connector_account::dsl;
#[cfg(feature = "v2")]
use crate::schema_v2::merchant_connector_account::dsl;
use crate::{
    errors,
    merchant_connector_account::{
        MerchantConnectorAccount, MerchantConnectorAccountNew,
        MerchantConnectorAccountUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl MerchantConnectorAccountNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantConnectorAccount> {
        generics::generic_insert(conn, self).await
    }
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccount {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.merchant_connector_id.to_owned(),
            merchant_connector_account,
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
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_connector(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_label: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_label.eq(connector_label.to_owned())),
        )
        .await
    }

    pub async fn find_by_profile_id_connector_name(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::connector_name.eq(connector_name.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_connector_name(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_name.eq(connector_name.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn find_by_merchant_id_merchant_connector_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(merchant_connector_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
    ) -> StorageResult<Vec<Self>> {
        if get_disabled {
            generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
                None,
                None,
                Some(dsl::created_at.asc()),
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

#[cfg(feature = "v2")]
impl MerchantConnectorAccount {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id.to_owned(),
            merchant_connector_account,
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

    pub async fn delete_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id.to_owned()))
            .await
    }

    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
    ) -> StorageResult<Vec<Self>> {
        if get_disabled {
            generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
                None,
                None,
                Some(dsl::created_at.asc()),
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

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    pub async fn list_enabled_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        connector_type: common_enums::ConnectorType,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::disabled.eq(false))
                .and(dsl::connector_type.eq(connector_type)),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
