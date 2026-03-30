//! Conversion implementations for PaymentAttempt

#[cfg(feature = "v1")]
use common_utils::types::ConnectorTransactionId;
use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    types::{
        keymanager::{self, KeyManagerState, ToEncryptable},
        ConnectorTransactionIdTrait, CreatedBy,
    },
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::payment_attempt::{EncryptedPaymentAttempt, PaymentAttempt},
    type_encryption::{crypto_operation, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for PaymentAttempt {
    type DstType = diesel_models::PaymentAttempt;
    type NewDstType = diesel_models::PaymentAttemptNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());
        let (connector_transaction_id, processor_transaction_data) = self
            .connector_transaction_id
            .map(ConnectorTransactionId::form_id_and_data)
            .map(|(txn_id, txn_data)| (Some(txn_id), txn_data))
            .unwrap_or((None, None));
        Ok(diesel_models::PaymentAttempt {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.net_amount.get_order_amount(),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            error_code: self.error_code,
            payment_token: self.payment_token,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(Into::into),
            error_reason: self.error_reason,
            multiple_capture_count: self.multiple_capture_count,
            connector_response_reference_id: self.connector_response_reference_id,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            net_amount: Some(self.net_amount.get_total_amount()),
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(Into::into),
            fingerprint_id: self.fingerprint_id,
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            charge_id: self.charge_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            card_network,
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            shipping_cost: self.net_amount.get_shipping_cost(),
            installment_data: self.installment_data,
            connector_mandate_detail: self.connector_mandate_detail,
            tokenization: self.tokenization,
            request_extended_authorization: self.request_extended_authorization,
            extended_authorization_applied: self.extended_authorization_applied,
            extended_authorization_last_applied_at: self.extended_authorization_last_applied_at,
            capture_before: self.capture_before,
            processor_transaction_data,
            card_discovery: self.card_discovery,
            charges: self.charges,
            issuer_error_code: self.issuer_error_code,
            issuer_error_message: self.issuer_error_message,
            setup_future_usage_applied: self.setup_future_usage_applied,
            error_details: self.error_details.map(Into::into),
            // Below fields are deprecated. Please add any new fields above this line.
            connector_transaction_data: None,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            routing_approach: self.routing_approach,
            connector_request_reference_id: self.connector_request_reference_id,
            network_transaction_id: self.network_transaction_id,
            is_overcapture_enabled: self.is_overcapture_enabled,
            network_details: self.network_details,
            is_stored_credential: self.is_stored_credential,
            authorized_amount: self.authorized_amount,
            encrypted_payment_method_data: self.encrypted_payment_method_data.map(Encryption::from),
            retry_type: self.retry_type,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let connector_transaction_id = storage_model
                .get_optional_connector_transaction_id()
                .cloned();
            let decrypted_data = crypto_operation(
                state,
                common_utils::type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentAttempt::to_encryptable(
                    EncryptedPaymentAttempt {
                        encrypted_payment_method_data: storage_model.encrypted_payment_method_data,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let decrypted_data = EncryptedPaymentAttempt::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let encrypted_payment_method_data = decrypted_data.encrypted_payment_method_data;
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id.clone(),
                attempt_id: storage_model.attempt_id,
                status: storage_model.status,
                net_amount: hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                    storage_model.amount,
                    storage_model.shipping_cost,
                    storage_model.order_tax_amount,
                    storage_model.surcharge_amount,
                    storage_model.tax_amount,
                    storage_model
                        .installment_data
                        .as_ref()
                        .and_then(|d| d.installment_interest),
                ),
                currency: storage_model.currency,
                save_to_locker: storage_model.save_to_locker,
                connector: storage_model.connector,
                error_message: storage_model.error_message,
                offer_amount: storage_model.offer_amount,
                payment_method_id: storage_model.payment_method_id,
                payment_method: storage_model.payment_method,
                connector_transaction_id,
                capture_method: storage_model.capture_method,
                capture_on: storage_model.capture_on,
                confirm: storage_model.confirm,
                authentication_type: storage_model.authentication_type,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                cancellation_reason: storage_model.cancellation_reason,
                amount_to_capture: storage_model.amount_to_capture,
                mandate_id: storage_model.mandate_id,
                browser_info: storage_model.browser_info,
                error_code: storage_model.error_code,
                payment_token: storage_model.payment_token,
                connector_metadata: storage_model.connector_metadata,
                payment_experience: storage_model.payment_experience,
                payment_method_type: storage_model.payment_method_type,
                payment_method_data: storage_model.payment_method_data,
                business_sub_label: storage_model.business_sub_label,
                straight_through_algorithm: storage_model.straight_through_algorithm,
                preprocessing_step_id: storage_model.preprocessing_step_id,
                mandate_details: storage_model.mandate_details.map(Into::into),
                error_reason: storage_model.error_reason,
                multiple_capture_count: storage_model.multiple_capture_count,
                connector_response_reference_id: storage_model.connector_response_reference_id,
                amount_capturable: storage_model.amount_capturable,
                updated_by: storage_model.updated_by,
                authentication_data: storage_model.authentication_data,
                encoded_data: storage_model.encoded_data,
                merchant_connector_id: storage_model.merchant_connector_id,
                unified_code: storage_model.unified_code,
                unified_message: storage_model.unified_message,
                external_three_ds_authentication_attempted: storage_model
                    .external_three_ds_authentication_attempted,
                authentication_connector: storage_model.authentication_connector,
                authentication_id: storage_model.authentication_id,
                mandate_data: storage_model.mandate_data.map(Into::into),
                payment_method_billing_address_id: storage_model.payment_method_billing_address_id,
                fingerprint_id: storage_model.fingerprint_id,
                charge_id: storage_model.charge_id,
                client_source: storage_model.client_source,
                client_version: storage_model.client_version,
                customer_acceptance: storage_model.customer_acceptance,
                profile_id: storage_model.profile_id,
                organization_id: storage_model.organization_id,
                connector_mandate_detail: storage_model.connector_mandate_detail,
                tokenization: storage_model.tokenization,
                request_extended_authorization: storage_model.request_extended_authorization,
                extended_authorization_applied: storage_model.extended_authorization_applied,
                extended_authorization_last_applied_at: storage_model
                    .extended_authorization_last_applied_at,
                capture_before: storage_model.capture_before,
                card_discovery: storage_model.card_discovery,
                charges: storage_model.charges,
                issuer_error_code: storage_model.issuer_error_code,
                issuer_error_message: storage_model.issuer_error_message,
                processor_merchant_id: storage_model
                    .processor_merchant_id
                    .unwrap_or(storage_model.merchant_id),
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                setup_future_usage_applied: storage_model.setup_future_usage_applied,
                routing_approach: storage_model.routing_approach,
                connector_request_reference_id: storage_model.connector_request_reference_id,
                debit_routing_savings: None,
                network_transaction_id: storage_model.network_transaction_id,
                is_overcapture_enabled: storage_model.is_overcapture_enabled,
                network_details: storage_model.network_details,
                is_stored_credential: storage_model.is_stored_credential,
                authorized_amount: storage_model.authorized_amount,
                encrypted_payment_method_data,
                error_details: storage_model.error_details.map(Into::into),
                retry_type: storage_model.retry_type,
                installment_data: storage_model.installment_data,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment attempt".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());
        Ok(diesel_models::PaymentAttemptNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.net_amount.get_order_amount(),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(Into::into),
            error_reason: self.error_reason,
            connector_response_reference_id: self.connector_response_reference_id,
            multiple_capture_count: self.multiple_capture_count,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            net_amount: Some(self.net_amount.get_total_amount()),
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(Into::into),
            fingerprint_id: self.fingerprint_id,
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            card_network,
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            shipping_cost: self.net_amount.get_shipping_cost(),
            connector_mandate_detail: self.connector_mandate_detail,
            tokenization: self.tokenization,
            request_extended_authorization: self.request_extended_authorization,
            extended_authorization_applied: self.extended_authorization_applied,
            extended_authorization_last_applied_at: self.extended_authorization_last_applied_at,
            capture_before: self.capture_before,
            card_discovery: self.card_discovery,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            setup_future_usage_applied: self.setup_future_usage_applied,
            routing_approach: self.routing_approach,
            connector_request_reference_id: self.connector_request_reference_id,
            network_transaction_id: self.network_transaction_id,
            network_details: self.network_details,
            is_stored_credential: self.is_stored_credential,
            authorized_amount: self.authorized_amount,
            encrypted_payment_method_data: self.encrypted_payment_method_data.map(Encryption::from),
            error_details: self.error_details.map(Into::into),
            retry_type: self.retry_type,
            installment_data: self.installment_data,
        })
    }
}
