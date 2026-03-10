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
        self.call_database(key_store, sub_new.insert(&conn)).await
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

    async fn update_subscription_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<DomainSubscription, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
