use common_utils::errors::CustomResult;
#[cfg(feature = "v2")]
use common_utils::{id_type, types::keymanager::KeyManagerState};
use diesel_models::enums as storage_enums;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptNew;
use hyperswitch_domain_models::{
    errors::StorageError,
    payments::payment_attempt::{PaymentAttempt, PaymentAttemptInterface, PaymentAttemptUpdate},
};

use super::MockDb;
#[cfg(feature = "v1")]
use crate::DataModelExt;

#[async_trait::async_trait]
impl PaymentAttemptInterface for MockDb {
    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        _payment_id: &common_utils::id_type::PaymentId,
        _merchant_id: &common_utils::id_type::MerchantId,
        _attempt_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filters_for_payments(
        &self,
        _pi: &[hyperswitch_domain_models::payments::PaymentIntent],
        _merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<
        hyperswitch_domain_models::payments::payment_attempt::PaymentListFilters,
        StorageError,
    > {
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _active_attempt_ids: &[String],
        _connector: Option<Vec<api_models::enums::Connector>>,
        _payment_method: Option<Vec<common_enums::PaymentMethod>>,
        _payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
        _authentication_type: Option<Vec<common_enums::AuthenticationType>>,
        _merchanat_connector_id: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
        _card_network: Option<Vec<storage_enums::CardNetwork>>,
        _card_discovery: Option<Vec<storage_enums::CardDiscovery>>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<i64, StorageError> {
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        _attempt_id: &str,
        _merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        _attempt_id: &id_type::GlobalAttemptId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempts_by_payment_intent_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _id: &id_type::GlobalPaymentId,
        _merchant_key_store: &MerchantKeyStore,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        _preprocessing_id: &str,
        _merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_txn_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_profile_id_connector_transaction_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        _profile_id: &id_type::ProfileId,
        _connector_transaction_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _payment_id: &common_utils::id_type::PaymentId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    #[allow(clippy::panic)]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;
        let time = common_utils::date_time::now();
        let payment_attempt = PaymentAttempt {
            payment_id: payment_attempt.payment_id,
            merchant_id: payment_attempt.merchant_id,
            attempt_id: payment_attempt.attempt_id,
            status: payment_attempt.status,
            net_amount: payment_attempt.net_amount,
            currency: payment_attempt.currency,
            save_to_locker: payment_attempt.save_to_locker,
            connector: payment_attempt.connector,
            error_message: payment_attempt.error_message,
            offer_amount: payment_attempt.offer_amount,
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
            charge_id: None,
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
            external_three_ds_authentication_attempted: payment_attempt
                .external_three_ds_authentication_attempted,
            authentication_connector: payment_attempt.authentication_connector,
            authentication_id: payment_attempt.authentication_id,
            mandate_data: payment_attempt.mandate_data,
            payment_method_billing_address_id: payment_attempt.payment_method_billing_address_id,
            fingerprint_id: payment_attempt.fingerprint_id,
            client_source: payment_attempt.client_source,
            client_version: payment_attempt.client_version,
            customer_acceptance: payment_attempt.customer_acceptance,
            organization_id: payment_attempt.organization_id,
            profile_id: payment_attempt.profile_id,
            connector_mandate_detail: payment_attempt.connector_mandate_detail,
            card_discovery: payment_attempt.card_discovery,
            charges: None,
        };
        payment_attempts.push(payment_attempt.clone());
        Ok(payment_attempt)
    }

    #[cfg(feature = "v2")]
    #[allow(clippy::panic)]
    async fn insert_payment_attempt(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        _payment_attempt: PaymentAttempt,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
    async fn update_payment_attempt(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        _this: PaymentAttempt,
        _payment_attempt: PaymentAttemptUpdate,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        _connector_transaction_id: &common_utils::types::ConnectorTransactionId,
        _payment_id: &common_utils::id_type::PaymentId,
        _merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let payment_attempts = self.payment_attempts.lock().await;

        Ok(payment_attempts
            .iter()
            .find(|payment_attempt| {
                payment_attempt.payment_id == *payment_id
                    && payment_attempt.merchant_id.eq(merchant_id)
            })
            .cloned()
            .unwrap())
    }

    #[cfg(feature = "v1")]
    #[allow(clippy::unwrap_used)]
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, StorageError> {
        let payment_attempts = self.payment_attempts.lock().await;

        Ok(payment_attempts
            .iter()
            .find(|payment_attempt| {
                payment_attempt.payment_id == *payment_id
                    && payment_attempt.merchant_id.eq(merchant_id)
                    && (payment_attempt.status == storage_enums::AttemptStatus::PartialCharged
                        || payment_attempt.status == storage_enums::AttemptStatus::Charged)
            })
            .cloned()
            .unwrap())
    }
}
