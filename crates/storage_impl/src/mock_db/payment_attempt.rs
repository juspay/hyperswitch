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
        /// Asynchronously finds a payment attempt by the given payment ID, merchant ID, attempt ID, and storage scheme.
    ///
    /// # Arguments
    ///
    /// * `_payment_id` - The payment ID to search for
    /// * `_merchant_id` - The merchant ID to search for
    /// * `_attempt_id` - The attempt ID to search for
    /// * `_storage_scheme` - The storage scheme to be used for the search
    ///
    /// # Returns
    ///
    /// * `CustomResult<PaymentAttempt, StorageError>` - A result containing either the found payment attempt or a storage error
    ///
    /// # Errors
    ///
    /// * Returns a `StorageError` with the reason for the error if the operation fails
    ///
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

        /// Asynchronously retrieves filters for payments based on the provided payment intents, merchant ID, and storage scheme.
    ///
    /// # Arguments
    ///
    /// * `pi` - A slice of payment intents to use for filtering payments.
    /// * `merchant_id` - The ID of the merchant for which the filters are being retrieved.
    /// * `storage_scheme` - The storage scheme to use for retrieving the filters.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the payment list filters if successful, otherwise a `StorageError` if an error occurs.
    ///
    /// # Errors
    ///
    /// This method returns a `StorageError::MockDbError` indicating a mock database error.
    ///
    async fn get_filters_for_payments(
        &self,
        _pi: &[data_models::payments::PaymentIntent],
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<data_models::payments::payment_attempt::PaymentListFilters, StorageError>
    {
        Err(StorageError::MockDbError)?
    }

        /// Asynchronously retrieves the total count of filtered payment attempts based on the provided criteria. 
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The ID of the merchant for which payment attempts are being filtered.
    /// * `_active_attempt_ids` - A list of active attempt IDs to filter the payment attempts.
    /// * `_connector` - An optional list of connectors to filter the payment attempts.
    /// * `_payment_method` - An optional list of payment methods to filter the payment attempts.
    /// * `_payment_method_type` - An optional list of payment method types to filter the payment attempts.
    /// * `_authentication_type` - An optional list of authentication types to filter the payment attempts.
    /// * `_storage_scheme` - The storage scheme used by the merchant.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the total count of filtered payment attempts as an `i64`, or a `StorageError` if the operation fails.
    ///
    /// # Errors
    ///
    /// Returns a `MockDbError` if the operation encounters a mock database error.
    ///
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

        /// Finds a payment attempt by its attempt ID and merchant ID in the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `_attempt_id` - The ID of the payment attempt to be found.
    /// * `_merchant_id` - The ID of the merchant associated with the payment attempt.
    /// * `_storage_scheme` - The storage scheme to be used for the search.
    ///
    /// # Returns
    ///
    /// * `CustomResult<PaymentAttempt, StorageError>` - A result containing either the found payment attempt or a storage error.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the function is called with a `MockDb` implementation.
    ///
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        _attempt_id: &str,
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

        /// Asynchronously finds a payment attempt by preprocessing ID and merchant ID using the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `_preprocessing_id` - The preprocessing ID of the payment attempt
    /// * `_merchant_id` - The ID of the merchant associated with the payment attempt
    /// * `_storage_scheme` - The storage scheme to be used for the operation
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `PaymentAttempt` if successful, or a `StorageError` if an error occurs
    ///
    /// # Errors
    ///
    /// An error of type `StorageError` is returned if the operation fails
    ///
    /// # Panics
    ///
    /// This method will panic if the storage type is `MockDb`
    ///
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        _preprocessing_id: &str,
        _merchant_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

        /// Asynchronously finds a payment attempt by the merchant ID and connector transaction ID using the specified storage scheme.
    ///
    /// # Arguments
    /// * `_merchant_id` - The ID of the merchant
    /// * `_connector_txn_id` - The ID of the connector transaction
    /// * `_storage_scheme` - The storage scheme to use
    ///
    /// # Returns
    /// The result of the operation, containing either a PaymentAttempt or a StorageError
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        _merchant_id: &str,
        _connector_txn_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
        /// Asynchronously finds payment attempts by merchant ID and payment ID using the specified storage scheme.
    /// 
    /// # Arguments
    /// * `_merchant_id` - The ID of the merchant
    /// * `_payment_id` - The ID of the payment
    /// * `_storage_scheme` - The storage scheme to use for retrieval
    /// 
    /// # Returns
    /// * `CustomResult<Vec<PaymentAttempt>, StorageError>` - A result containing a vector of PaymentAttempt objects, or a StorageError if an error occurs
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
        /// Inserts a new payment attempt into the storage, populating derived fields and returning the inserted payment attempt.
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;
        #[allow(clippy::as_conversions)]
        let id = payment_attempts.len() as i32;
        let time = common_utils::date_time::now();
        let payment_attempt = payment_attempt.populate_derived_fields();
        let payment_attempt = PaymentAttempt {
            id,
            payment_id: payment_attempt.payment_id,
            merchant_id: payment_attempt.merchant_id,
            attempt_id: payment_attempt.attempt_id,
            status: payment_attempt.status,
            amount: payment_attempt.amount,
            net_amount: payment_attempt.net_amount,
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
            unified_code: payment_attempt.unified_code,
            unified_message: payment_attempt.unified_message,
        };
        payment_attempts.push(payment_attempt.clone());
        Ok(payment_attempt)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
        /// Updates a payment attempt with the given attempt ID and returns the updated payment attempt.
    ///
    /// # Arguments
    ///
    /// * `this` - The original payment attempt to be updated
    /// * `payment_attempt` - The updated payment attempt data
    /// * `_storage_scheme` - The storage scheme to use for the update
    ///
    /// # Returns
    ///
    /// The updated payment attempt, or a `StorageError` if the update fails
    ///
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

        /// Asynchronously finds a payment attempt by the given connector transaction ID, payment ID, and merchant ID using the specified storage scheme.
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
        /// Asynchronously finds the last successful payment attempt by the given payment ID and merchant ID.
    ///
    /// # Arguments
    ///
    /// * `payment_id` - The ID of the payment to search for.
    /// * `merchant_id` - The ID of the merchant to search for.
    /// * `_storage_scheme` - The storage scheme to use for the merchant.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the last successful `PaymentAttempt` found, or a `StorageError` if an error occurs.
    ///
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
    #[allow(clippy::unwrap_used)]
        /// Asynchronously finds and returns the last successful or partially captured payment attempt based on the provided payment ID and merchant ID, using the specified storage scheme.
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
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
                    && (payment_attempt.status == storage_enums::AttemptStatus::PartialCharged
                        || payment_attempt.status == storage_enums::AttemptStatus::Charged)
            })
            .cloned()
            .unwrap())
    }
}
