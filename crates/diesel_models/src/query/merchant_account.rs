use diesel::{associations::HasTable, ExpressionMethods, Table};

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::merchant_account::dsl::{self, merchant_id as dsl_identifier};
#[cfg(feature = "v2")]
use crate::schema_v2::merchant_account::dsl::{self, id as dsl_identifier};
use crate::{
    errors,
    merchant_account::{MerchantAccount, MerchantAccountNew, MerchantAccountUpdateInternal},
    PgPooledConn, StorageResult,
};

impl MerchantAccountNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantAccount> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantAccount {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_account: MerchantAccountUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.get_id().to_owned(),
            merchant_account,
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

    pub async fn update_with_specific_fields(
        conn: &PgPooledConn,
        identifier: &common_utils::id_type::MerchantId,
        merchant_account: MerchantAccountUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl_identifier.eq(identifier.to_owned()),
            merchant_account,
        )
        .await
    }

    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        identifier: &common_utils::id_type::MerchantId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl_identifier.eq(identifier.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        identifier: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl_identifier.eq(identifier.to_owned()),
        )
        .await
    }

    pub async fn find_by_publishable_key(
        conn: &PgPooledConn,
        publishable_key: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::publishable_key.eq(publishable_key.to_owned()),
        )
        .await
    }

    pub async fn list_by_organization_id(
        conn: &PgPooledConn,
        organization_id: &common_utils::id_type::OrganizationId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::organization_id.eq(organization_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn list_multiple_merchant_accounts(
        conn: &PgPooledConn,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(conn, dsl_identifier.eq_any(merchant_ids), None, None, None)
        .await
    }
    pub async fn list_all_merchant_accounts(
        conn: &PgPooledConn,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl_identifier.ne_all(vec![""]),
            limit.map(i64::from),
            offset.map(i64::from),
            None,
        )
        .await
    }

    pub async fn update_all_merchant_accounts(
        conn: &PgPooledConn,
        merchant_account: MerchantAccountUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl_identifier.ne_all(vec![""]),
            merchant_account,
        )
        .await
    }
}
