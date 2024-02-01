use diesel::{associations::HasTable, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{
    organization::*, query::generics, schema::organization::dsl, PgPooledConn, StorageResult,
};

impl OrganizationNew {
    #[instrument(skip(conn))]
        /// Inserts the current organization into the database using the provided database connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection.
    /// 
    /// # Returns
    /// 
    /// The result of the insertion operation, wrapped in a `StorageResult` enum.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Organization> {
            generics::generic_insert(conn, self).await
        }
}

impl Organization {
        /// Asynchronously finds a record in the database by the specified organization ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection.
    /// * `org_id` - The organization ID to search for.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the found record, if any.
    /// 
    pub async fn find_by_org_id(conn: &PgPooledConn, org_id: String) -> StorageResult<Self> {
            generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, dsl::org_id.eq(org_id))
                .await
        }

        /// Asynchronously updates an organization record in the database based on the organization ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection to execute the update query on.
    /// * `org_id` - The ID of the organization to update.
    /// * `update` - The updated information for the organization.
    ///
    /// # Returns
    ///
    /// The result of the update operation, wrapped in a `StorageResult` enum.
    pub async fn update_by_org_id(
        conn: &PgPooledConn,
        org_id: String,
        update: OrganizationUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::org_id.eq(org_id),
            OrganizationUpdateInternal::from(update),
        )
        .await
    }
}
