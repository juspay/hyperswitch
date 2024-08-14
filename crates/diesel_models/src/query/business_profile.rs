use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};

use super::generics;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
use crate::schema::business_profile::dsl;
#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
use crate::schema_v2::business_profile::dsl;
use crate::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdateInternal},
    errors, PgPooledConn, StorageResult,
};

impl BusinessProfileNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BusinessProfile> {
        generics::generic_insert(conn, self).await
    }
}

impl BusinessProfile {
    pub async fn update_by_profile_id(
        self,
        conn: &PgPooledConn,
        business_profile: BusinessProfileUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.profile_id.clone(),
            business_profile,
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

    pub async fn find_by_profile_id(conn: &PgPooledConn, profile_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id_profile_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_profile_name_merchant_id(
        conn: &PgPooledConn,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_name
                .eq(profile_name.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn list_business_profile_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
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
    }

    pub async fn delete_by_profile_id_merchant_id(
        conn: &PgPooledConn,
        profile_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }
}
