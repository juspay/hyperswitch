use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{ConnectorResponse, ConnectorResponseNew, ConnectorResponseUpdate},
};

#[async_trait::async_trait]
pub trait IConnectorResponse {
    async fn insert_connector_response(
        &self,
        connector_response: ConnectorResponseNew,
    ) -> CustomResult<ConnectorResponse, errors::StorageError>;
    async fn find_connector_response_by_payment_id_merchant_id_txn_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<ConnectorResponse, errors::StorageError>;
    async fn update_connector_response(
        &self,
        this: ConnectorResponse,
        payment_attempt: ConnectorResponseUpdate,
    ) -> CustomResult<ConnectorResponse, errors::StorageError>;
}

#[async_trait::async_trait]
impl IConnectorResponse for Store {
    async fn insert_connector_response(
        &self,
        connector_response: ConnectorResponseNew,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        connector_response.insert(&conn).await
    }

    async fn find_connector_response_by_payment_id_merchant_id_txn_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        ConnectorResponse::find_by_payment_id_and_merchant_id_transaction_id(
            &conn,
            payment_id,
            merchant_id,
            txn_id,
        )
        .await
    }

    async fn update_connector_response(
        &self,
        this: ConnectorResponse,
        connector_response_update: ConnectorResponseUpdate,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        this.update(&conn, connector_response_update).await
    }
}
