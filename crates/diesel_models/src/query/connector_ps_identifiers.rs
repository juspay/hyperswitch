use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use super::generics;
use crate::{
    connector_ps_identifiers::{
        ConnectorPsIdentifierUpdateInternal, ConnectorPsIdentifiers, ConnectorPsIdentifiersNew,
        ConnectorPsIdentifiersUpdate,
    },
    errors,
    schema::connector_ps_identifiers::dsl,
    PgPooledConn, StorageResult,
};

impl ConnectorPsIdentifiersNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ConnectorPsIdentifiers> {
        generics::generic_insert(conn, self).await
    }
}

impl ConnectorPsIdentifiers {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        connector_ps_ids_update: ConnectorPsIdentifiersUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(self.merchant_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            ConnectorPsIdentifierUpdateInternal::from(connector_ps_ids_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(mut connector_ps_identifiers) => connector_ps_identifiers
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }
}
