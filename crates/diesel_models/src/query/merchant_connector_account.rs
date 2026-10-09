use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

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
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantConnectorAccountNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<MerchantConnectorAccount> {
        generics::generic_insert(conn, self).await
    }
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccount {
    pub async fn update(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
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
        conn: &DatabaseConnectionWithContext<'_>,
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
        conn: &DatabaseConnectionWithContext<'_>,
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
        conn: &DatabaseConnectionWithContext<'_>,
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

    pub async fn find_by_merchant_id_merchant_connector_id(
        conn: &DatabaseConnectionWithContext<'_>,
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

    /// Every merchant connector account of a merchant, including disabled ones, ordered by
    /// `created_at` ascending. The other merchant-scoped list queries are row-subsets of
    /// this one, so it doubles as the cached superset they project from.
    pub async fn list_by_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    /// Every merchant connector account of a merchant's profile, including disabled ones,
    /// ordered by `created_at` ascending. The other profile-scoped list queries are
    /// row-subsets of this one, so it doubles as the cached superset they project from.
    ///
    /// Scoped by merchant as well as profile so the cached superset can be keyed by both.
    /// Profile ids are globally unique, so the extra predicate does not change the rows
    /// returned; it keeps one merchant's cache entries from ever being addressable by
    /// another's key.
    pub async fn list_by_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccount {
    pub async fn update(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
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
        conn: &DatabaseConnectionWithContext<'_>,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id.to_owned()))
            .await
    }

    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    /// Every merchant connector account of a merchant, including disabled ones, ordered by
    /// `created_at` ascending. The other merchant-scoped list queries are row-subsets of
    /// this one, so it doubles as the cached superset they project from.
    pub async fn list_by_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    /// Every merchant connector account of a merchant's profile, including disabled ones,
    /// ordered by `created_at` ascending. The other profile-scoped list queries are
    /// row-subsets of this one, so it doubles as the cached superset they project from.
    ///
    /// Scoped by merchant as well as profile so the cached superset can be keyed by both.
    /// Profile ids are globally unique, so the extra predicate does not change the rows
    /// returned; it keeps one merchant's cache entries from ever being addressable by
    /// another's key.
    pub async fn list_by_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
