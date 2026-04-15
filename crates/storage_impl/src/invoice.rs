use common_utils::errors::CustomResult;
pub use diesel_models::invoice::Invoice as DieselInvoice;
use error_stack::{report, ResultExt};
pub use hyperswitch_domain_models::{
    invoice::{Invoice as DomainInvoice, InvoiceInterface, InvoiceUpdate as DomainInvoiceUpdate},
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

use crate::{
    behaviour::Conversion, connection, errors::StorageError, kv_router_store::KVRouterStore,
    DatabaseStore, MockDb, RouterStore,
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
        self.call_database_new(key_store, inv_new.insert(&conn))
            .await
    }

    #[instrument(skip_all)]
    async fn find_invoice_by_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<DomainInvoice, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        self.call_database_new(
            key_store,
            DieselInvoice::find_invoice_by_id_invoice_id(&conn, invoice_id),
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
        self.call_database_new(
            key_store,
            DieselInvoice::update_invoice_entry(&conn, invoice_id, inv_new),
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
            .find_resources_new(
                key_store,
                DieselInvoice::list_invoices_by_subscription_id(
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
        self.find_optional_resource_new(
            key_store,
            DieselInvoice::get_invoice_by_subscription_id_connector_invoice_id(
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
use common_utils::{
    errors::ValidationError,
    types::keymanager::{Identifier, KeyManagerState},
};
use hyperswitch_masking::Secret;

#[async_trait::async_trait]

impl Conversion for DomainInvoice {
    type DstType = diesel_models::invoice::Invoice;
    type NewDstType = diesel_models::invoice::InvoiceNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = common_utils::date_time::now();
        Ok(DieselInvoice {
            id: self.id,
            subscription_id: self.subscription_id,
            merchant_id: self.merchant_id,
            profile_id: self.profile_id,
            merchant_connector_id: self.merchant_connector_id,
            payment_intent_id: self.payment_intent_id,
            payment_method_id: self.payment_method_id,
            customer_id: self.customer_id,
            amount: self.amount,
            currency: self.currency.to_string(),
            status: self.status,
            provider_name: self.provider_name,
            metadata: None,
            created_at: now,
            modified_at: now,
            connector_invoice_id: self.connector_invoice_id,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            subscription_id: item.subscription_id,
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            merchant_connector_id: item.merchant_connector_id,
            payment_intent_id: item.payment_intent_id,
            payment_method_id: item.payment_method_id,
            customer_id: item.customer_id,
            amount: item.amount,
            currency: item.currency,
            status: item.status,
            provider_name: item.provider_name,
            metadata: item.metadata,
            connector_invoice_id: item.connector_invoice_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceNew::new(
            self.subscription_id,
            self.merchant_id,
            self.profile_id,
            self.merchant_connector_id,
            self.payment_intent_id,
            self.payment_method_id,
            self.customer_id,
            self.amount,
            self.currency.to_string(),
            self.status,
            self.provider_name,
            None,
            self.connector_invoice_id,
        ))
    }
}

#[async_trait::async_trait]
impl Conversion for DomainInvoiceUpdate {
    type DstType = diesel_models::invoice::InvoiceUpdate;
    type NewDstType = diesel_models::invoice::InvoiceUpdate;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            connector_invoice_id: self.connector_invoice_id,
            modified_at: self.modified_at,
            payment_intent_id: self.payment_intent_id,
            amount: self.amount,
            currency: self.currency,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            status: item.status,
            payment_method_id: item.payment_method_id,
            connector_invoice_id: item.connector_invoice_id,
            modified_at: item.modified_at,
            payment_intent_id: item.payment_intent_id,
            amount: item.amount,
            currency: item.currency,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            connector_invoice_id: self.connector_invoice_id,
            modified_at: self.modified_at,
            payment_intent_id: self.payment_intent_id,
            amount: self.amount,
            currency: self.currency,
        })
    }
}
