use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use crate::{
    errors, gsm::*, query::generics, schema::gateway_status_map::dsl, PgPooledConn, StorageResult,
};

impl GatewayStatusMappingNew {
        /// Asynchronously inserts the current instance of GatewayStatusMap into the database using the provided PostgreSQL pooled connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a PostgreSQL pooled connection
    /// 
    /// # Returns
    /// 
    /// The result of the insertion operation, wrapped in a `StorageResult` enum
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<GatewayStatusMap> {
        generics::generic_insert(conn, self).await
    }
}

impl GatewayStatusMap {
        /// Asynchronously finds a record in the database based on the provided parameters.
    /// 
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled database connection
    /// * `connector` - The connector value to search for
    /// * `flow` - The flow value to search for
    /// * `sub_flow` - The sub_flow value to search for
    /// * `code` - The code value to search for
    /// * `message` - The message value to search for
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    pub async fn find(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
        )
        .await
    }

        /// Asynchronously retrieves the decision from the storage for the given parameters.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `connector` - The connector for the decision
    /// * `flow` - The flow for the decision
    /// * `sub_flow` - The sub-flow for the decision
    /// * `code` - The code for the decision
    /// * `message` - The message for the decision
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the retrieved decision as a `String`
    pub async fn retrieve_decision(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<String> {
        Self::find(conn, connector, flow, sub_flow, code, message)
            .await
            .map(|item| item.decision)
    }

        /// Updates the gateway status mapping entry in the database with the provided parameters.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection
    /// * `connector` - The connector name
    /// * `flow` - The flow name
    /// * `sub_flow` - The sub flow name
    /// * `code` - The status code
    /// * `message` - The status message
    /// * `gsm` - The updated gateway status mapping
    ///
    /// # Returns
    ///
    /// The updated gateway status mapping entry as a result, or a `DatabaseError::NotFound` if the entry is not found.
    pub async fn update(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        gsm: GatewayStatusMappingUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            GatewayStatusMapperUpdateInternal,
            _,
            _,
        >(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
            gsm.into(),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating gsm entry")
        })
    }

        /// Deletes a record from the database table based on the provided parameters. 
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled database connection
    /// * `connector` - The connector value to match
    /// * `flow` - The flow value to match
    /// * `sub_flow` - The sub_flow value to match
    /// * `code` - The code value to match
    /// * `message` - The message value to match
    /// 
    /// # Returns
    /// A `StorageResult` indicating whether the delete operation was successful
    pub async fn delete(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
        )
        .await
    }
}
