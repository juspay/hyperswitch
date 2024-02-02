use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{
    enums,
    query::generics,
    schema::dashboard_metadata::dsl,
    user::dashboard_metadata::{
        DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate,
        DashboardMetadataUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl DashboardMetadataNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a DashboardMetadata instance into the database using the provided PgPooledConn connection
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DashboardMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl DashboardMetadata {
        /// Updates the dashboard metadata for a specific user, merchant, and organization in the database.
    /// If a user_id is provided, it updates the record for that specific user. If user_id is None, it updates the record where user_id is null.
    pub async fn update(
        conn: &PgPooledConn,
        user_id: Option<String>,
        merchant_id: String,
        org_id: String,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: DashboardMetadataUpdate,
    ) -> StorageResult<Self> {
        let predicate = dsl::merchant_id
            .eq(merchant_id.to_owned())
            .and(dsl::org_id.eq(org_id.to_owned()))
            .and(dsl::data_key.eq(data_key.to_owned()));

        if let Some(uid) = user_id {
            generics::generic_update_with_unique_predicate_get_result::<
                <Self as HasTable>::Table,
                _,
                _,
                _,
            >(
                conn,
                predicate.and(dsl::user_id.eq(uid)),
                DashboardMetadataUpdateInternal::from(dashboard_metadata_update),
            )
            .await
        } else {
            generics::generic_update_with_unique_predicate_get_result::<
                <Self as HasTable>::Table,
                _,
                _,
                _,
            >(
                conn,
                predicate.and(dsl::user_id.is_null()),
                DashboardMetadataUpdateInternal::from(dashboard_metadata_update),
            )
            .await
        }
    }

        /// Asynchronously finds user-scoped dashboard metadata based on the provided user ID, merchant ID, organization ID, and data types. 
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection
    /// * `user_id` - The ID of the user
    /// * `merchant_id` - The ID of the merchant
    /// * `org_id` - The ID of the organization
    /// * `data_types` - A vector of dashboard metadata types
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing a vector of the found dashboard metadata
    ///
    pub async fn find_user_scoped_dashboard_metadata(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
        org_id: String,
        data_types: Vec<enums::DashboardMetadata>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::user_id
            .eq(user_id)
            .and(dsl::merchant_id.eq(merchant_id))
            .and(dsl::org_id.eq(org_id))
            .and(dsl::data_key.eq_any(data_types));

        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            predicate,
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

        /// Asynchronously finds the metadata for the merchant scoped dashboard. It takes the database connection, merchant ID, organization ID, and a vector of data types as input parameters. It then constructs a predicate based on the input parameters and uses the generic_filter function to filter the results based on the predicate. Finally, it returns a StorageResult containing the filtered metadata.
    pub async fn find_merchant_scoped_dashboard_metadata(
        conn: &PgPooledConn,
        merchant_id: String,
        org_id: String,
        data_types: Vec<enums::DashboardMetadata>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::user_id
            .is_null()
            .and(dsl::merchant_id.eq(merchant_id))
            .and(dsl::org_id.eq(org_id))
            .and(dsl::data_key.eq_any(data_types));

        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            predicate,
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

        /// Deletes user scoped dashboard metadata by merchant id from the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection.
    /// * `user_id` - A string representing the user id.
    /// * `merchant_id` - A string representing the merchant id.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` indicating whether the deletion was successful.
    pub async fn delete_user_scoped_dashboard_metadata_by_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }
}
