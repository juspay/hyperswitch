use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::business_profile::dsl::{self, profile_id as dsl_identifier};
#[cfg(feature = "v2")]
use crate::schema_v2::business_profile::dsl::{self, id as dsl_identifier};
use crate::{
    business_profile::{Profile, ProfileNew, ProfileUpdateInternal},
    errors, PgPooledConn, StorageResult,
};

impl ProfileNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Profile> {
        generics::generic_insert(conn, self).await
    }
}

impl Profile {
    pub async fn update_by_profile_id(
        self,
        conn: &PgPooledConn,
        business_profile: ProfileUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.get_id().to_owned(),
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

    pub async fn find_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl_identifier.eq(profile_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id_profile_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl_identifier.eq(profile_id.to_owned())),
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

    pub async fn list_profile_by_merchant_id(
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
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl_identifier
                .eq(profile_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }
}
