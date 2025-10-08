use error_stack::report;
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[async_trait::async_trait]
pub trait InvoiceInterface {
    async fn insert_invoice_entry(
        &self,
        invoice_new: storage::invoice::InvoiceNew,
    ) -> CustomResult<storage::Invoice, errors::StorageError>;

    async fn find_invoice_by_invoice_id(
        &self,
        invoice_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError>;

    async fn update_invoice_entry(
        &self,
        invoice_id: String,
        data: storage::invoice::InvoiceUpdate,
    ) -> CustomResult<storage::Invoice, errors::StorageError>;

    async fn get_latest_invoice_for_subscription(
        &self,
        subscription_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError>;
}

#[async_trait::async_trait]
impl InvoiceInterface for Store {
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        invoice_new: storage::invoice::InvoiceNew,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        invoice_new
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        invoice_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Invoice::find_invoice_by_id_invoice_id(&conn, invoice_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        invoice_id: String,
        data: storage::invoice::InvoiceUpdate,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Invoice::update_invoice_entry(&conn, invoice_id, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_latest_invoice_for_subscription(
        &self,
        subscription_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Invoice::list_invoices_by_subscription_id(
            &conn,
            subscription_id.clone(),
            Some(1),
            None,
            false,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .map(|e| e.last().cloned())?
        .ok_or(report!(errors::StorageError::ValueNotFound(format!(
            "Invoice not found for subscription_id: {}",
            subscription_id
        ))))
    }
}

#[async_trait::async_trait]
impl InvoiceInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        _invoice_new: storage::invoice::InvoiceNew,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_invoice_by_invoice_id(
        &self,
        _invoice_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_invoice_entry(
        &self,
        _invoice_id: String,
        _data: storage::invoice::InvoiceUpdate,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn get_latest_invoice_for_subscription(
        &self,
        _subscription_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl InvoiceInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        invoice_new: storage::invoice::InvoiceNew,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        self.diesel_store.insert_invoice_entry(invoice_new).await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        invoice_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        self.diesel_store
            .find_invoice_by_invoice_id(invoice_id)
            .await
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        invoice_id: String,
        data: storage::invoice::InvoiceUpdate,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        self.diesel_store
            .update_invoice_entry(invoice_id, data)
            .await
    }

    async fn get_latest_invoice_for_subscription(
        &self,
        subscription_id: String,
    ) -> CustomResult<storage::Invoice, errors::StorageError> {
        self.diesel_store
            .get_latest_invoice_for_subscription(subscription_id)
            .await
    }
}
