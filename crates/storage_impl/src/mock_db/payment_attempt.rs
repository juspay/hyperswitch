use api_models::enums::{AuthenticationType, Connector, PaymentMethod, PaymentMethodType};
use common_utils::errors::CustomResult;
use data_models::{
    errors::StorageError,
    payments::payment_attempt::{
        PaymentAttempt, PaymentAttemptInterface, PaymentAttemptNew, PaymentAttemptUpdate,
    },
};
use diesel_models::enums as storage_enums;

use super::MockDb;
use crate::DataModelExt;

#[async_trait::async_trait]
impl PaymentAttemptInterface for MockDb {
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _attempt_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn get_filters_for_payments(
        &self,
        _pi: &[data_models::payments::PaymentIntent],
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<data_models::payments::payment_attempt::PaymentListFilters, StorageError>
    {
        Err(StorageError::MockDbError)?
    }

    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        _merchant_id: &str,
        _active_attempt_ids: &[String],
        _connector: Option<Vec<Connector>>,
        _payment_method: Option<Vec<PaymentMethod>>,
        _payment_method_type: Option<Vec<PaymentMethodType>>,
        _authentication_type: Option<Vec<AuthenticationType>>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<i64, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        _attempt_id: &str,
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        _preprocessing_id: &str,
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        _merchant_id: &str,
        _connector_txn_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        _merchant_id: &str,
        _payment_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;
        #[allow(clippy::as_conversions)]
        let id = payment_attempts.len() as i32;
        let time = common_utils::date_time::now();

        let payment_attempt = PaymentAttempt {
            id,
            payment_id: payment_attempt.payment_id,
            merchant_id: payment_attempt.merchant_id,
            attempt_id: payment_attempt.attempt_id,
            status: payment_attempt.status,
            amount: payment_attempt.amount,
            currency: payment_attempt.currency,
            save_to_locker: payment_attempt.save_to_locker,
            connector: payment_attempt.connector,
            error_message: payment_attempt.error_message,
            offer_amount: payment_attempt.offer_amount,
            surcharge_amount: payment_attempt.surcharge_amount,
            tax_amount: payment_attempt.tax_amount,
            payment_method_id: payment_attempt.payment_method_id,
            payment_method: payment_attempt.payment_method,
            connector_transaction_id: None,
            capture_method: payment_attempt.capture_method,
            capture_on: payment_attempt.capture_on,
            confirm: payment_attempt.confirm,
            authentication_type: payment_attempt.authentication_type,
            created_at: payment_attempt.created_at.unwrap_or(time),
            modified_at: payment_attempt.modified_at.unwrap_or(time),
            last_synced: payment_attempt.last_synced,
            cancellation_reason: payment_attempt.cancellation_reason,
            amount_to_capture: payment_attempt.amount_to_capture,
            mandate_id: None,
            browser_info: None,
            payment_token: None,
            error_code: payment_attempt.error_code,
            connector_metadata: None,
            payment_experience: payment_attempt.payment_experience,
            payment_method_type: payment_attempt.payment_method_type,
            payment_method_data: payment_attempt.payment_method_data,
            business_sub_label: payment_attempt.business_sub_label,
            straight_through_algorithm: payment_attempt.straight_through_algorithm,
            mandate_details: payment_attempt.mandate_details,
            preprocessing_step_id: payment_attempt.preprocessing_step_id,
            error_reason: payment_attempt.error_reason,
            multiple_capture_count: payment_attempt.multiple_capture_count,
            connector_response_reference_id: None,
            amount_capturable: payment_attempt.amount_capturable,
            updated_by: storage_scheme.to_string(),
            authentication_data: payment_attempt.authentication_data,
            encoded_data: payment_attempt.encoded_data,
            merchant_connector_id: payment_attempt.merchant_connector_id,
        };
        payment_attempts.push(payment_attempt.clone());
        Ok(payment_attempt)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;

        let item = payment_attempts
            .iter_mut()
            .find(|item| item.attempt_id == this.attempt_id)
            .unwrap();

        *item = PaymentAttempt::from_storage_model(
            payment_attempt
                .to_storage_model()
                .apply_changeset(this.to_storage_model()),
        );

        Ok(item.clone())
    }

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        _connector_transaction_id: &str,
        _payment_id: &str,
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let payment_attempts = self.payment_attempts.lock().await;

        Ok(payment_attempts
            .iter()
            .find(|payment_attempt| {
                payment_attempt.payment_id == payment_id
                    && payment_attempt.merchant_id == merchant_id
            })
            .cloned()
            .unwrap())
    }
}
