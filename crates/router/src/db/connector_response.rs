use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{ConnectorResponse, ConnectorResponseNew, ConnectorResponseUpdate},
};

#[async_trait::async_trait]
pub trait ConnectorResponseInterface {
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
impl ConnectorResponseInterface for super::Store {
    async fn insert_connector_response(
        &self,
        connector_response: ConnectorResponseNew,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        connector_response.insert(&conn).await
    }

    async fn find_connector_response_by_payment_id_merchant_id_txn_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
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
        let conn = pg_connection(&self.master_pool.conn).await;
        this.update(&conn, connector_response_update).await
    }
}

#[async_trait::async_trait]
impl ConnectorResponseInterface for MockDb {
    async fn insert_connector_response(
        &self,
        new: ConnectorResponseNew,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = ConnectorResponse {
            id: connector_response.len() as i32,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            txn_id: new.txn_id,
            created_at: new.created_at,
            modified_at: new.modified_at,
            connector_name: new.connector_name,
            connector_transaction_id: new.connector_transaction_id,
            authentication_data: new.authentication_data,
            encoded_data: new.encoded_data,
        };
        connector_response.push(response.clone());
        Ok(response)
    }

    async fn find_connector_response_by_payment_id_merchant_id_txn_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _txn_id: &str,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        todo!()
    }

    async fn update_connector_response(
        &self,
        this: ConnectorResponse,
        connector_response_update: ConnectorResponseUpdate,
    ) -> CustomResult<ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = connector_response
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();
        *response = connector_response_update.apply_changeset(response.clone());
        Ok(response.clone())
    }
}
