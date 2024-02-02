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
    #[instrument(skip(conn))]
        /// Asynchronously inserts a BusinessProfile into the database using the provided PgPooledConn connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a PgPooledConn connection to the database.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `BusinessProfile` if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BusinessProfile> {
        generics::generic_insert(conn, self).await
    }
}

impl BusinessProfile {
    #[instrument(skip(conn))]
        /// Asynchronously updates a record in the database based on the profile ID, using the provided business profile update data.
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
        /// Asynchronously finds a record by the given profile ID in the database using the provided database connection.
    pub async fn find_by_profile_id(conn: &PgPooledConn, profile_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record by the given profile name and merchant ID in the database.
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

        /// Retrieves a list of business profiles associated with a specific merchant ID from the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection
    /// * `merchant_id` - A string slice representing the merchant ID for which to retrieve the business profiles
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing a vector of the retrieved business profiles, if successful
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

        /// Deletes a record from the database based on the given profile_id and merchant_id.
    /// Returns a boolean indicating whether the deletion was successful.
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
