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
pub trait SubscriptionInterface {
    async fn insert_subscription_entry(
        &self,
        subscription_new: storage::subscription::SubscriptionNew,
    ) -> CustomResult<storage::Subscription, errors::StorageError>;

    async fn find_by_merchant_id_subscription_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<storage::Subscription, errors::StorageError>;

    async fn update_subscription_entry(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: storage::SubscriptionUpdate,
    ) -> CustomResult<storage::Subscription, errors::StorageError>;
}

#[async_trait::async_trait]
impl SubscriptionInterface for Store {
    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        subscription_new: storage::subscription::SubscriptionNew,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        subscription_new
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_by_merchant_id_subscription_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Subscription::find_by_merchant_id_subscription_id(
            &conn,
            merchant_id,
            subscription_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: storage::SubscriptionUpdate,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Subscription::update_subscription_entry(&conn, merchant_id, subscription_id, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl SubscriptionInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        _subscription_new: storage::subscription::SubscriptionNew,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_by_merchant_id_subscription_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_subscription_entry(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _data: storage::SubscriptionUpdate,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl SubscriptionInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_subscription_entry(
        &self,
        subscription_new: storage::subscription::SubscriptionNew,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        self.diesel_store
            .insert_subscription_entry(subscription_new)
            .await
    }

    #[instrument(skip_all)]
    async fn find_by_merchant_id_subscription_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        self.diesel_store
            .find_by_merchant_id_subscription_id(merchant_id, subscription_id)
            .await
    }

    #[instrument(skip_all)]
    async fn update_subscription_entry(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: storage::SubscriptionUpdate,
    ) -> CustomResult<storage::Subscription, errors::StorageError> {
        self.diesel_store
            .update_subscription_entry(merchant_id, subscription_id, data)
            .await
    }
}
