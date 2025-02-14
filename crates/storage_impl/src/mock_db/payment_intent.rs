use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    behaviour::Conversion,
    errors::StorageError,
    merchant_key_store::MerchantKeyStore,
    payments::{
        payment_intent::{PaymentIntentInterface, PaymentIntentUpdate},
        PaymentIntent,
    },
};

use super::MockDb;

#[async_trait::async_trait]
impl PaymentIntentInterface for MockDb {
    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intent_by_constraints(
        &self,
        _state: &KeyManagerState,
        _merchant_id: &common_utils::id_type::MerchantId,
        _filters: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        _state: &KeyManagerState,
        _merchant_id: &common_utils::id_type::MerchantId,
        _time_range: &common_utils::types::TimeRange,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn get_intent_status_with_count(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        _time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::IntentStatus, i64)>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_payment_intents_attempt(
        &self,
        _state: &KeyManagerState,
        _merchant_id: &common_utils::id_type::MerchantId,
        _constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(
            PaymentIntent,
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        )>,
        StorageError,
    > {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_intent(
        &self,
        _state: &KeyManagerState,
        new: PaymentIntent,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        payment_intents.push(new.clone());
        Ok(new)
    }

    #[cfg(feature = "v1")]
    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_intent(
        &self,
        state: &KeyManagerState,
        this: PaymentIntent,
        update: PaymentIntentUpdate,
        key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter_mut()
            .find(|item| item.get_id() == this.get_id() && item.merchant_id == this.merchant_id)
            .unwrap();

        let diesel_payment_intent_update = diesel_models::PaymentIntentUpdate::from(update);
        let diesel_payment_intent = payment_intent
            .clone()
            .convert()
            .await
            .change_context(StorageError::EncryptionError)?;

        *payment_intent = PaymentIntent::convert_back(
            state,
            diesel_payment_intent_update.apply_changeset(diesel_payment_intent),
            key_store.key.get_inner(),
            key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)?;

        Ok(payment_intent.clone())
    }

    #[cfg(feature = "v2")]
    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_intent(
        &self,
        state: &KeyManagerState,
        this: PaymentIntent,
        update: PaymentIntentUpdate,
        key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let payment_intents = self.payment_intents.lock().await;

        Ok(payment_intents
            .iter()
            .find(|payment_intent| {
                payment_intent.get_id() == payment_id && payment_intent.merchant_id.eq(merchant_id)
            })
            .cloned()
            .unwrap())
    }

    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_id(
        &self,
        _state: &KeyManagerState,
        id: &common_utils::id_type::GlobalPaymentId,
        _merchant_key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter()
            .find(|payment_intent| payment_intent.get_id() == id)
            .ok_or(StorageError::ValueNotFound(
                "PaymentIntent not found".to_string(),
            ))?;

        Ok(payment_intent.clone())
    }
    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_merchant_reference_id_profile_id(
        &self,
        _state: &KeyManagerState,
        merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
        profile_id: &common_utils::id_type::ProfileId,
        _merchant_key_store: &MerchantKeyStore,
        _storage_scheme: &common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter()
            .find(|payment_intent| {
                payment_intent.merchant_reference_id.as_ref() == Some(merchant_reference_id)
                    && payment_intent.profile_id.eq(profile_id)
            })
            .ok_or(StorageError::ValueNotFound(
                "PaymentIntent not found".to_string(),
            ))?;

        Ok(payment_intent.clone())
    }
}
