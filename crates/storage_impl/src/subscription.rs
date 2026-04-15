use common_utils::errors::CustomResult;
pub use diesel_models::subscription::Subscription as DieselSubscription;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    subscription::{
        Subscription as DomainSubscription, SubscriptionInterface,
        SubscriptionUpdate as DomainSubscriptionUpdate,
    },
};
use router_env::{instrument, tracing};

use crate::{
    behaviour::Conversion, connection, errors::StorageError, kv_router_store::KVRouterStore,
    DatabaseStore, MockDb, RouterStore,
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
        self.call_database_new(key_store, sub_new.insert(&conn))
            .await
    }
    #[instrument(skip_all)]
    async fn find_by_merchant_id_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<DomainSubscription, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        self.call_database_new(
            key_store,
            DieselSubscription::find_by_merchant_id_subscription_id(
                &conn,
                merchant_id,
                subscription_id,
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
        self.call_database_new(
            key_store,
            DieselSubscription::update_subscription_entry(
                &conn,
                merchant_id,
                subscription_id,
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
        self.find_resources_new(
            key_store,
            DieselSubscription::list_by_merchant_id_profile_id(
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

    async fn update_subscription_entry(
        &self,
        _key_store: &MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _subscription_id: String,
        _data: DomainSubscriptionUpdate,
    ) -> CustomResult<DomainSubscription, StorageError> {
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

use common_utils::{
    errors::ValidationError,
    pii::SecretSerdeValue,
    types::keymanager::{self, KeyManagerState},
};
use hyperswitch_masking::{ExposeInterface, Secret};

#[async_trait::async_trait]
impl Conversion for DomainSubscription {
    type DstType = diesel_models::subscription::Subscription;
    type NewDstType = diesel_models::subscription::SubscriptionNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = common_utils::date_time::now();
        Ok(diesel_models::subscription::Subscription {
            id: self.id,
            status: self.status,
            billing_processor: self.billing_processor,
            payment_method_id: self.payment_method_id,
            merchant_connector_id: self.merchant_connector_id,
            client_secret: self.client_secret,
            connector_subscription_id: self.connector_subscription_id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            metadata: self.metadata.map(|m| m.expose()),
            created_at: now,
            modified_at: now,
            profile_id: self.profile_id,
            merchant_reference_id: self.merchant_reference_id,
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            status: item.status,
            billing_processor: item.billing_processor,
            payment_method_id: item.payment_method_id,
            merchant_connector_id: item.merchant_connector_id,
            client_secret: item.client_secret,
            connector_subscription_id: item.connector_subscription_id,
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            metadata: item.metadata.map(SecretSerdeValue::new),
            created_at: item.created_at,
            modified_at: item.modified_at,
            profile_id: item.profile_id,
            merchant_reference_id: item.merchant_reference_id,
            plan_id: item.plan_id,
            item_price_id: item.item_price_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionNew::new(
            self.id,
            self.status,
            self.billing_processor,
            self.payment_method_id,
            self.merchant_connector_id,
            self.client_secret,
            self.connector_subscription_id,
            self.merchant_id,
            self.customer_id,
            self.metadata,
            self.profile_id,
            self.merchant_reference_id,
            self.plan_id,
            self.item_price_id,
        ))
    }
}

#[async_trait::async_trait]
impl Conversion for DomainSubscriptionUpdate {
    type DstType = diesel_models::subscription::SubscriptionUpdate;
    type NewDstType = diesel_models::subscription::SubscriptionUpdate;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionUpdate {
            connector_subscription_id: self.connector_subscription_id,
            payment_method_id: self.payment_method_id,
            status: self.status,
            modified_at: self.modified_at,
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            connector_subscription_id: item.connector_subscription_id,
            payment_method_id: item.payment_method_id,
            status: item.status,
            modified_at: item.modified_at,
            plan_id: item.plan_id,
            item_price_id: item.item_price_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionUpdate {
            connector_subscription_id: self.connector_subscription_id,
            payment_method_id: self.payment_method_id,
            status: self.status,
            modified_at: self.modified_at,
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
        })
    }
}
