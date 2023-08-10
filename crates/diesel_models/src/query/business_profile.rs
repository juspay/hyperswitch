use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdateInternal},
    errors,
    schema::business_profile::dsl,
    PgPooledConn, StorageResult,
};

impl BusinessProfileNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BusinessProfile> {
        generics::generic_insert(conn, self).await
    }
}

impl BusinessProfile {
    #[instrument(skip(conn))]
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

    #[instrument(skip(conn))]
    pub async fn find_by_profile_id(conn: &PgPooledConn, profile_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
        )
        .await
    }

    pub async fn delete_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            &conn,
            dsl::profile_id.eq(profile_id.to_owned()),
        )
        .await
    }
}
