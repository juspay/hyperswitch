use common_utils::errors::CustomResult;
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::payouts::{
    payout_attempt::{
        PayoutAttempt, PayoutAttemptInterface, PayoutAttemptNew, PayoutAttemptUpdate,
    },
    payouts::Payouts,
};

use super::MockDb;
use crate::errors::StorageError;

#[async_trait::async_trait]
impl PayoutAttemptInterface for MockDb {
    type Error = StorageError;
    async fn update_payout_attempt(
        &self,
        this: &PayoutAttempt,
        payout_attempt_update: PayoutAttemptUpdate,
        _payouts: &Payouts,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        let mut payout_attempts = self.payout_attempt.lock().await;
        
        let payout_attempt = payout_attempts
            .iter_mut()
            .find(|payout_attempt| {
                payout_attempt.merchant_id == this.merchant_id 
                    && payout_attempt.payout_attempt_id == this.payout_attempt_id
            })
            .ok_or(StorageError::ValueNotFound(
                "Payout attempt not found for update".to_string()
            ))?;

        // Apply the update fields
        if let Some(status) = payout_attempt_update.status {
            payout_attempt.status = status;
        }
        if let Some(amount) = payout_attempt_update.amount {
            payout_attempt.amount = amount;
        }
        if let Some(connector_payout_id) = payout_attempt_update.connector_payout_id {
            payout_attempt.connector_payout_id = connector_payout_id;
        }
        if let Some(payout_token) = payout_attempt_update.payout_token {
            payout_attempt.payout_token = payout_token;
        }
        if let Some(unified_code) = payout_attempt_update.unified_code {
            payout_attempt.unified_code = unified_code;
        }
        if let Some(unified_message) = payout_attempt_update.unified_message {
            payout_attempt.unified_message = unified_message;
        }
        if let Some(connector_metadata) = payout_attempt_update.connector_metadata {
            payout_attempt.connector_metadata = connector_metadata;
        }
        if let Some(error_message) = payout_attempt_update.error_message {
            payout_attempt.error_message = error_message;
        }
        if let Some(error_code) = payout_attempt_update.error_code {
            payout_attempt.error_code = error_code;
        }
        if let Some(is_eligible) = payout_attempt_update.is_eligible {
            payout_attempt.is_eligible = is_eligible;
        }
        
        // Update the last_modified_at timestamp
        payout_attempt.last_modified_at = common_utils::date_time::now();

        Ok(payout_attempt.clone())
    }

    async fn insert_payout_attempt(
        &self,
        payout_attempt: PayoutAttemptNew,
        payouts: &Payouts,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        let mut payout_attempts = self.payout_attempt.lock().await;
        let payout_attempt = PayoutAttempt {
            id: common_utils::generate_id(common_utils::consts::ID_LENGTH, "poa"),
            payout_id: payouts.payout_id.clone(),
            merchant_id: payouts.merchant_id.clone(),
            payout_attempt_id: payout_attempt.payout_attempt_id,
            status: payout_attempt.status,
            amount: payout_attempt.amount,
            currency: payout_attempt.currency,
            connector: payout_attempt.connector,
            connector_payout_id: payout_attempt.connector_payout_id,
            payout_token: payout_attempt.payout_token,
            confirm: payout_attempt.confirm,
            payout_method_data: payout_attempt.payout_method_data,
            payout_method_type: payout_attempt.payout_method_type,
            authentication_type: payout_attempt.authentication_type,
            business_country: payout_attempt.business_country,
            business_label: payout_attempt.business_label,
            auto_fulfill: payout_attempt.auto_fulfill,
            client_secret: payout_attempt.client_secret,
            return_url: payout_attempt.return_url,
            entity_type: payout_attempt.entity_type,
            recurring: payout_attempt.recurring,
            metadata: payout_attempt.metadata,
            unified_code: payout_attempt.unified_code,
            unified_message: payout_attempt.unified_message,
            created_at: payout_attempt.created_at,
            last_modified_at: payout_attempt.last_modified_at,
            connector_metadata: payout_attempt.connector_metadata,
            routing_info: payout_attempt.routing_info,
            error_message: payout_attempt.error_message,
            error_code: payout_attempt.error_code,
            is_eligible: payout_attempt.is_eligible,
            profile_id: payout_attempt.profile_id,
            merchant_connector_id: payout_attempt.merchant_connector_id,
            priority: payout_attempt.priority,
        };
        
        payout_attempts.push(payout_attempt.clone());
        Ok(payout_attempt)
    }

    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_attempt_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        let payout_attempts = self.payout_attempt.lock().await;
        
        payout_attempts
            .iter()
            .find(|payout_attempt| {
                payout_attempt.merchant_id == *merchant_id 
                    && payout_attempt.payout_attempt_id == payout_attempt_id
            })
            .cloned()
            .ok_or(StorageError::ValueNotFound(
                "Payout attempt not found".to_string()
            ).into())
    }

    async fn find_payout_attempt_by_merchant_id_connector_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_payout_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        let payout_attempts = self.payout_attempt.lock().await;
        
        payout_attempts
            .iter()
            .find(|payout_attempt| {
                payout_attempt.merchant_id == *merchant_id 
                    && payout_attempt.connector_payout_id.as_deref() == Some(connector_payout_id)
            })
            .cloned()
            .ok_or(StorageError::ValueNotFound(
                "Payout attempt not found".to_string()
            ).into())
    }

    async fn get_filters_for_payouts(
        &self,
        _payouts: &[Payouts],
        _merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<
        hyperswitch_domain_models::payouts::payout_attempt::PayoutListFilters,
        StorageError,
    > {
        Err(StorageError::MockDbError)?
    }

    async fn find_payout_attempt_by_merchant_id_merchant_order_reference_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_order_reference_id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<PayoutAttempt, StorageError> {
        let payout_attempts = self.payout_attempt.lock().await;
        let payouts = self.payouts.lock().await;
        
        // First find the payout with the matching merchant_order_reference_id
        let matching_payout = payouts
            .iter()
            .find(|payout| {
                payout.merchant_id == *merchant_id 
                    && payout.merchant_order_reference_id.as_deref() == Some(merchant_order_reference_id)
            });
            
        if let Some(payout) = matching_payout {
            // Then find the payout attempt for this payout
            payout_attempts
                .iter()
                .find(|payout_attempt| {
                    payout_attempt.merchant_id == *merchant_id 
                        && payout_attempt.payout_id == payout.payout_id
                })
                .cloned()
                .ok_or(StorageError::ValueNotFound(
                    "Payout attempt not found for merchant order reference".to_string()
                ).into())
        } else {
            Err(StorageError::ValueNotFound(
                "Payout not found for merchant order reference".to_string()
            ).into())
        }
    }
}
