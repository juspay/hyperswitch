use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics;
use crate::{
    connector_response::{
        ConnectorResponse, ConnectorResponseNew, ConnectorResponseUpdate,
        ConnectorResponseUpdateInternal,
    },
    errors,
    schema::connector_response::dsl,
    PgPooledConn, StorageResult,
};

impl ConnectorResponseNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ConnectorResponse> {
        generics::generic_insert(conn, self).await
    }
}

impl ConnectorResponse {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        connector_response: ConnectorResponseUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            ConnectorResponseUpdateInternal::from(connector_response),
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
    pub async fn find_by_payment_id_and_merchant_id_transaction_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
        transaction_id: &str,
    ) -> StorageResult<ConnectorResponse> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()).and(
                dsl::payment_id
                    .eq(payment_id.to_owned())
                    .and(dsl::txn_id.eq(transaction_id.to_owned())),
            ),
        )
        .await
    }
}
