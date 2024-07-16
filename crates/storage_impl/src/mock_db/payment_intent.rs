use common_utils::errors::CustomResult;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    behaviour::Conversion,
    errors::StorageError,
    merchant_key_store::MerchantKeyStore,
    payments::{
        payment_attempt::PaymentAttempt,
        payment_intent::{PaymentIntentInterface, PaymentIntentUpdate},
        PaymentIntent,
    },
};

use super::MockDb;

#[async_trait::async_trait]
impl PaymentIntentInterface for MockDb {
    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        _merchant_id: &str,
        _filters: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        _merchant_id: &str,
        _time_range: &api_models::payments::TimeRange,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        _merchant_id: &str,
        _constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        _merchant_id: &str,
        _constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_intent(
        &self,
        new: PaymentIntent,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        payment_intents.push(new.clone());
        Ok(new)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        update: PaymentIntentUpdate,
        key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter_mut()
            .find(|item| item.payment_id == this.payment_id && item.merchant_id == this.merchant_id)
            .unwrap();

        let diesel_payment_intent_update = diesel_models::PaymentIntentUpdate::from(update);
        let diesel_payment_intent = payment_intent
            .clone()
            .convert()
            .await
            .change_context(StorageError::EncryptionError)?;

        *payment_intent = PaymentIntent::convert_back(
            diesel_payment_intent_update.apply_changeset(diesel_payment_intent),
            key_store.key.get_inner(),
        )
        .await
        .change_context(StorageError::DecryptionError)?;

        Ok(payment_intent.clone())
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _key_store: &MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let payment_intents = self.payment_intents.lock().await;

        Ok(payment_intents
            .iter()
            .find(|payment_intent| {
                payment_intent.payment_id == payment_id && payment_intent.merchant_id == merchant_id
            })
            .cloned()
            .unwrap())
    }

    async fn get_active_payment_attempt(
        &self,
        payment: &mut PaymentIntent,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, StorageError> {
        match payment.active_attempt.clone() {
            hyperswitch_domain_models::RemoteStorageObject::ForeignID(id) => {
                let attempts = self.payment_attempts.lock().await;
                let attempt = attempts
                    .iter()
                    .find(|pa| pa.attempt_id == id && pa.merchant_id == payment.merchant_id)
                    .ok_or(StorageError::ValueNotFound("Attempt not found".to_string()))?;

                payment.active_attempt = attempt.clone().into();
                Ok(attempt.clone())
            }
            hyperswitch_domain_models::RemoteStorageObject::Object(pa) => Ok(pa.clone()),
        }
    }
}
