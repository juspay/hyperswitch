//! Conversion implementations for Subscription and SubscriptionUpdate

use common_utils::{
    date_time,
    errors::{CustomResult, ValidationError},
    pii::SecretSerdeValue,
    types::keymanager::{self, KeyManagerState},
};
use hyperswitch_domain_models::subscription::{Subscription, SubscriptionUpdate};
use hyperswitch_masking::{ExposeInterface, Secret};

use crate::behaviour::Conversion;

#[async_trait::async_trait]
impl Conversion for Subscription {
    type DstType = diesel_models::subscription::Subscription;
    type NewDstType = diesel_models::subscription::SubscriptionNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = date_time::now();
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
impl Conversion for SubscriptionUpdate {
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
