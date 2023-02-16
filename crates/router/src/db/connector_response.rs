use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{self, enums},
};

#[async_trait::async_trait]
pub trait ConnectorResponseInterface {
    async fn insert_connector_response(
        &self,
        connector_response: storage::ConnectorResponseNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError>;

    async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError>;

    async fn update_connector_response(
        &self,
        this: storage::ConnectorResponse,
        payment_attempt: storage::ConnectorResponseUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConnectorResponseInterface for Store {
    async fn insert_connector_response(
        &self,
        connector_response: storage::ConnectorResponseNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        connector_response
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ConnectorResponse::find_by_payment_id_merchant_id_attempt_id(
            &conn,
            payment_id,
            merchant_id,
            attempt_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn update_connector_response(
        &self,
        this: storage::ConnectorResponse,
        connector_response_update: storage::ConnectorResponseUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        this.update(&conn, connector_response_update)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl ConnectorResponseInterface for MockDb {
    async fn insert_connector_response(
        &self,
        new: storage::ConnectorResponseNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = storage::ConnectorResponse {
            #[allow(clippy::as_conversions)]
            id: connector_response.len() as i32,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            attempt_id: new.attempt_id,
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

    async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    // safety: interface only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_connector_response(
        &self,
        this: storage::ConnectorResponse,
        connector_response_update: storage::ConnectorResponseUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = connector_response
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();
        *response = connector_response_update.apply_changeset(response.clone());
        Ok(response.clone())
    }
}
