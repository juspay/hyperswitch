use common_utils::errors::CustomResult;
use data_models::{
    errors::StorageError,
    payments::{
        payment_attempt::PaymentAttempt,
        payment_intent::{PaymentIntentInterface, PaymentIntentNew, PaymentIntentUpdate},
        PaymentIntent,
    },
};
use diesel_models::enums as storage_enums;
use error_stack::{IntoReport, ResultExt};

use super::MockDb;
use crate::DataModelExt;

#[async_trait::async_trait]
impl PaymentIntentInterface for MockDb {
    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        _merchant_id: &str,
        _filters: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
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
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        _merchant_id: &str,
        _constraints: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        _merchant_id: &str,
        _constraints: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let time = common_utils::date_time::now();
        let payment_intent = PaymentIntent {
            #[allow(clippy::as_conversions)]
            id: payment_intents
                .len()
                .try_into()
                .into_report()
                .change_context(StorageError::MockDbError)?,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            status: new.status,
            amount: new.amount,
            currency: new.currency,
            amount_captured: new.amount_captured,
            customer_id: new.customer_id,
            description: new.description,
            return_url: new.return_url,
            metadata: new.metadata,
            connector_id: new.connector_id,
            shipping_address_id: new.shipping_address_id,
            billing_address_id: new.billing_address_id,
            statement_descriptor_name: new.statement_descriptor_name,
            statement_descriptor_suffix: new.statement_descriptor_suffix,
            created_at: new.created_at.unwrap_or(time),
            modified_at: new.modified_at.unwrap_or(time),
            last_synced: new.last_synced,
            setup_future_usage: new.setup_future_usage,
            off_session: new.off_session,
            client_secret: new.client_secret,
            business_country: new.business_country,
            business_label: new.business_label,
            active_attempt: new.active_attempt,
            order_details: new.order_details,
            allowed_payment_method_types: new.allowed_payment_method_types,
            connector_metadata: new.connector_metadata,
            feature_metadata: new.feature_metadata,
            attempt_count: new.attempt_count,
            profile_id: new.profile_id,
            merchant_decision: new.merchant_decision,
            payment_link_id: new.payment_link_id,
            payment_confirm_source: new.payment_confirm_source,
            updated_by: storage_scheme.to_string(),
            surcharge_applicable: new.surcharge_applicable,
        };
        payment_intents.push(payment_intent.clone());
        Ok(payment_intent)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        update: PaymentIntentUpdate,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentIntent, StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();
        *payment_intent = PaymentIntent::from_storage_model(
            update
                .to_storage_model()
                .apply_changeset(this.to_storage_model()),
        );
        Ok(payment_intent.clone())
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
            data_models::RemoteStorageObject::ForeignID(id) => {
                let attempts = self.payment_attempts.lock().await;
                let attempt = attempts
                    .iter()
                    .find(|pa| pa.attempt_id == id && pa.merchant_id == payment.merchant_id)
                    .ok_or(StorageError::ValueNotFound("Attempt not found".to_string()))?;

                payment.active_attempt = attempt.clone().into();
                Ok(attempt.clone())
            }
            data_models::RemoteStorageObject::Object(pa) => Ok(pa.clone()),
        }
    }
}
