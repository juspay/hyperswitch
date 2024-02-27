use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdateInternal},
    errors,
    schema::business_profile::dsl,
    PgPooledConn, StorageResult,
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

    pub async fn find_by_profile_name_merchant_id(
        conn: &PgPooledConn,
        profile_name: &str,
        merchant_id: &str,
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
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq(merchant_id.to_string()),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn delete_by_profile_id_merchant_id(
        conn: &PgPooledConn,
        profile_id: &str,
        merchant_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_string())),
        )
        .await
    }
}
