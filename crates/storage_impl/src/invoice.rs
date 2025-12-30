use common_utils::errors::CustomResult;
pub use diesel_models::invoice::Invoice;
use error_stack::{report, ResultExt};
pub use hyperswitch_domain_models::{
    behaviour::Conversion,
    invoice::{Invoice as DomainInvoice, InvoiceInterface, InvoiceUpdate as DomainInvoiceUpdate},
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

use crate::{
    connection, errors::StorageError, kv_router_store::KVRouterStore, DatabaseStore, MockDb,
    RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> InvoiceInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let inv_new = invoice_new
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(key_store, inv_new.insert(&conn)).await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        self.call_database(
            key_store,
            Invoice::find_invoice_by_id_invoice_id(&conn, invoice_id),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let inv_new = data
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(
            key_store,
            Invoice::update_invoice_entry(&conn, invoice_id, inv_new),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn get_latest_invoice_for_subscription(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let invoices: Vec<DomainInvoice> = self
            .find_resources(
                key_store,
                Invoice::list_invoices_by_subscription_id(
                    &conn,
                    subscription_id.clone(),
                    Some(1),
                    None,
                    false,
                ),
            )
            .await?;

        invoices
            .last()
            .cloned()
            .ok_or(report!(StorageError::ValueNotFound(format!(
                "Invoice not found for subscription_id: {}",
                subscription_id
            ))))
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_subscription_id_connector_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
        connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> CustomResult<Option<DomainInvoice>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        self.find_optional_resource(
            key_store,
            Invoice::get_invoice_by_subscription_id_connector_invoice_id(
                &conn,
                subscription_id,
                connector_invoice_id,
            ),
        )
        .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> InvoiceInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store
            .insert_invoice_entry(key_store, invoice_new)
            .await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store
            .find_invoice_by_invoice_id(key_store, invoice_id)
            .await
    }

    #[instrument(skip_all)]
    async fn update_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store
            .update_invoice_entry(key_store, invoice_id, data)
            .await
    }

    #[instrument(skip_all)]
    async fn get_latest_invoice_for_subscription(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        self.router_store
            .get_latest_invoice_for_subscription(key_store, subscription_id)
            .await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_subscription_id_connector_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
        connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> CustomResult<Option<DomainInvoice>, StorageError> {
        self.router_store
            .find_invoice_by_subscription_id_connector_invoice_id(
                key_store,
                subscription_id,
                connector_invoice_id,
            )
            .await
    }
}

#[async_trait::async_trait]
impl InvoiceInterface for MockDb {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_invoice_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _invoice_new: DomainInvoice,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_invoice_by_invoice_id(
        &self,
        _key_store: &MerchantKeyStore,
        _invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_invoice_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _invoice_id: String,
        _data: DomainInvoiceUpdate,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn get_latest_invoice_for_subscription(
        &self,
        _key_store: &MerchantKeyStore,
        _subscription_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_invoice_by_subscription_id_connector_invoice_id(
        &self,
        _key_store: &MerchantKeyStore,
        _subscription_id: String,
        _connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> CustomResult<Option<DomainInvoice>, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
