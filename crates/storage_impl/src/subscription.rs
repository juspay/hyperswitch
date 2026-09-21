use common_utils::errors::CustomResult;
pub use diesel_models::subscription::Subscription;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    behaviour::Conversion,
    merchant_key_store::MerchantKeyStore,
    subscription::{
        Subscription as DomainSubscription, SubscriptionInterface,
        SubscriptionUpdate as DomainSubscriptionUpdate,
    },
};
use router_env::{instrument, tracing};

use crate::{
    connection, errors::StorageError, kv_router_store::KVRouterStore, DatabaseStore, MockDb,
    RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> SubscriptionInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        key_store: &MerchantKeyStore,
        subscription_new: DomainSubscription,
    ) -> CustomResult<DomainSubscription, StorageError> {
        let sub_new = subscription_new
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        Box::pin(self.call_database(key_store, sub_new.insert(&conn))).await
    }
    #[instrument(skip_all)]
    async fn find_by_merchant_id_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(
            key_store,
            Subscription::find_by_merchant_id_subscription_id(&conn, merchant_id, subscription_id),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn find_by_merchant_id_connector_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        connector_subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(
            key_store,
            Subscription::find_by_merchant_id_connector_subscription_id(
                &conn,
                merchant_id,
                merchant_connector_id,
                connector_subscription_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<DomainSubscription, StorageError> {
        let sub_new = data
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.call_database(
            key_store,
            Subscription::update_subscription_entry(&conn, merchant_id, subscription_id, sub_new),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry_if_status(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        expected_status: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        let sub_new = data
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.find_optional_resource(
            key_store,
            Subscription::update_subscription_entry_if_status(
                &conn,
                merchant_id,
                subscription_id,
                expected_status,
                sub_new,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry_if_status_and_invoice_status(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        expected_status: String,
        invoice_id: common_utils::id_type::InvoiceId,
        expected_invoice_status: common_enums::InvoiceStatus,
        billing_period_end: time::PrimitiveDateTime,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        let sub_new = data
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.find_optional_resource(
            key_store,
            Subscription::update_subscription_entry_if_status_and_invoice_status(
                &conn,
                merchant_id,
                subscription_id,
                expected_status,
                invoice_id,
                expected_invoice_status,
                billing_period_end,
                sub_new,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn bind_connector_subscription_id_if_unset_or_equal(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        connector_subscription_id: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        let sub_new = data
            .construct_new()
            .await
            .change_context(StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        self.find_optional_resource(
            key_store,
            Subscription::bind_connector_subscription_id_if_unset_or_equal(
                &conn,
                merchant_id,
                subscription_id,
                connector_subscription_id,
                sub_new,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn list_by_merchant_id_profile_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<DomainSubscription>, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        self.find_resources(
            key_store,
            Subscription::list_by_merchant_id_profile_id(
                &conn,
                merchant_id,
                profile_id,
                limit,
                offset,
            ),
        )
        .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> SubscriptionInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        key_store: &MerchantKeyStore,
        subscription_new: DomainSubscription,
    ) -> CustomResult<DomainSubscription, StorageError> {
        self.router_store
            .insert_subscription_entry(key_store, subscription_new)
            .await
    }
    #[instrument(skip_all)]
    async fn find_by_merchant_id_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        self.router_store
            .find_by_merchant_id_subscription_id(key_store, merchant_id, subscription_id)
            .await
    }

    async fn find_by_merchant_id_connector_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        connector_subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        self.router_store
            .find_by_merchant_id_connector_subscription_id(
                key_store,
                merchant_id,
                merchant_connector_id,
                connector_subscription_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<DomainSubscription, StorageError> {
        self.router_store
            .update_subscription_entry(key_store, merchant_id, subscription_id, data)
            .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry_if_status(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        expected_status: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        self.router_store
            .update_subscription_entry_if_status(
                key_store,
                merchant_id,
                subscription_id,
                expected_status,
                data,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry_if_status_and_invoice_status(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        expected_status: String,
        invoice_id: common_utils::id_type::InvoiceId,
        expected_invoice_status: common_enums::InvoiceStatus,
        billing_period_end: time::PrimitiveDateTime,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        self.router_store
            .update_subscription_entry_if_status_and_invoice_status(
                key_store,
                merchant_id,
                subscription_id,
                expected_status,
                invoice_id,
                expected_invoice_status,
                billing_period_end,
                data,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn bind_connector_subscription_id_if_unset_or_equal(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        connector_subscription_id: String,
        data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        self.router_store
            .bind_connector_subscription_id_if_unset_or_equal(
                key_store,
                merchant_id,
                subscription_id,
                connector_subscription_id,
                data,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_by_merchant_id_profile_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<DomainSubscription>, StorageError> {
        self.router_store
            .list_by_merchant_id_profile_id(key_store, merchant_id, profile_id, limit, offset)
            .await
    }
}

#[async_trait::async_trait]
impl SubscriptionInterface for MockDb {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _subscription_new: DomainSubscription,
    ) -> CustomResult<DomainSubscription, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_by_merchant_id_subscription_id(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_by_merchant_id_connector_subscription_id(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        _connector_subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_subscription_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<DomainSubscription, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_subscription_entry_if_status(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _expected_status: String,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_subscription_entry_if_status_and_invoice_status(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _expected_status: String,
        _invoice_id: common_utils::id_type::InvoiceId,
        _expected_invoice_status: common_enums::InvoiceStatus,
        _billing_period_end: time::PrimitiveDateTime,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn bind_connector_subscription_id_if_unset_or_equal(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _connector_subscription_id: String,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<Option<DomainSubscription>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn list_by_merchant_id_profile_id(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> CustomResult<Vec<DomainSubscription>, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
