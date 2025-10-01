pub use diesel_models::invoice::Invoice;

use crate::MockDb;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use crate::{connection, errors::StorageError,DatabaseStore,RouterStore,kv_router_store::KVRouterStore,};
use common_utils::{errors::CustomResult,types::keymanager::KeyManagerState};

pub use hyperswitch_domain_models::{
    behaviour::Conversion, invoice::Invoice as DomainInvoice, merchant_key_store::MerchantKeyStore,
    invoice::InvoiceInterface, invoice::InvoiceUpdate as DomainInvoiceUpdate,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> InvoiceInterface for RouterStore<T> {
    type Error = StorageError;
    
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let inv_new = invoice_new
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(state, key_store,inv_new.insert(&conn)).await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        self.call_database(state, key_store, Invoice::find_invoice_by_id_invoice_id(&conn, invoice_id)).await
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let inv_new = data.construct_new().await.change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(state, key_store,Invoice::update_invoice_entry(&conn, invoice_id, inv_new)).await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> InvoiceInterface for KVRouterStore<T> {
    type Error = StorageError;
    
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store.insert_invoice_entry(state, key_store, invoice_new).await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store.find_invoice_by_invoice_id(state, key_store, invoice_id).await
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store.update_invoice_entry(state, key_store, invoice_id, data).await
    }
}

#[async_trait::async_trait]
impl InvoiceInterface for MockDb {
    type Error = StorageError;
    
    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_invoice_by_invoice_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_invoice_entry(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _invoice_id: String,
        _data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }
}