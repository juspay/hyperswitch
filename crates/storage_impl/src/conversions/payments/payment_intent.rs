//! Conversion implementations for PaymentIntent

use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::{
        keymanager::{self, KeyManagerState, ToEncryptable},
        CreatedBy,
    },
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::{EncryptedPaymentIntent, PaymentIntent },
    type_encryption::{crypto_operation, CryptoOperation},
};

use hyperswitch_domain_models::RemoteStorageObject;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for PaymentIntent {
    type DstType = diesel_models::PaymentIntent;
    type NewDstType = diesel_models::PaymentIntentNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::PaymentIntent {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: None, // deprecated
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: None,
            split_payments: self.split_payments,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
            request_extended_authorization: self.request_extended_authorization,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            platform_merchant_id: None,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            force_3ds_challenge: self.force_3ds_challenge,
            force_3ds_challenge_trigger: self.force_3ds_challenge_trigger,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            extended_return_url: self.return_url,
            is_payment_id_from_merchant: self.is_payment_id_from_merchant,
            payment_channel: self.payment_channel,
            tax_status: self.tax_status,
            discount_amount: self.discount_amount,
            order_date: self.order_date,
            shipping_amount_tax: self.shipping_amount_tax,
            duty_amount: self.duty_amount,
            enable_partial_authorization: self.enable_partial_authorization,
            enable_overcapture: self.enable_overcapture,
            mit_category: self.mit_category,
            billing_descriptor: self.billing_descriptor,
            tokenization: self.tokenization,
            partner_merchant_identifier_details: self.partner_merchant_identifier_details,
            state_metadata: self.state_metadata,
            installment_options: self
                .installment_options
                .map(common_types::payments::InstallmentOptions),
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
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(
                    EncryptedPaymentIntent::to_encryptable(
                        EncryptedPaymentIntent {
                            billing_details: storage_model.billing_details,
                            shipping_details: storage_model.shipping_details,
                            customer_details: storage_model.customer_details,
                        },
                    ),
                ),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentIntent::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id.clone(),
                status: storage_model.status,
                amount: storage_model.amount,
                currency: storage_model.currency,
                amount_captured: storage_model.amount_captured,
                customer_id: storage_model.customer_id,
                description: storage_model.description,
                return_url: storage_model
                    .extended_return_url
                    .or(storage_model.return_url),
                metadata: storage_model.metadata,
                connector_id: storage_model.connector_id,
                shipping_address_id: storage_model.shipping_address_id,
                billing_address_id: storage_model.billing_address_id,
                statement_descriptor_name: storage_model.statement_descriptor_name,
                statement_descriptor_suffix: storage_model.statement_descriptor_suffix,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                setup_future_usage: storage_model.setup_future_usage,
                off_session: storage_model.off_session,
                client_secret: storage_model.client_secret,
                active_attempt: RemoteStorageObject::ForeignID(storage_model.active_attempt_id),
                business_country: storage_model.business_country,
                business_label: storage_model.business_label,
                order_details: storage_model.order_details,
                allowed_payment_method_types: storage_model.allowed_payment_method_types,
                connector_metadata: storage_model.connector_metadata,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                merchant_decision: storage_model.merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                payment_confirm_source: storage_model.payment_confirm_source,
                updated_by: storage_model.updated_by,
                surcharge_applicable: storage_model.surcharge_applicable,
                request_incremental_authorization: storage_model.request_incremental_authorization,
                incremental_authorization_allowed: storage_model.incremental_authorization_allowed,
                authorization_count: storage_model.authorization_count,
                fingerprint_id: storage_model.fingerprint_id,
                session_expiry: storage_model.session_expiry,
                request_external_three_ds_authentication: storage_model
                    .request_external_three_ds_authentication,
                split_payments: storage_model.split_payments,
                frm_metadata: storage_model.frm_metadata,
                shipping_cost: storage_model.shipping_cost,
                tax_details: storage_model.tax_details,
                customer_details: data.customer_details,
                billing_details: data.billing_details,
                merchant_order_reference_id: storage_model.merchant_order_reference_id,
                shipping_details: data.shipping_details,
                is_payment_processor_token_flow: storage_model.is_payment_processor_token_flow,
                organization_id: storage_model.organization_id,
                skip_external_tax_calculation: storage_model.skip_external_tax_calculation,
                request_extended_authorization: storage_model.request_extended_authorization,
                psd2_sca_exemption_type: storage_model.psd2_sca_exemption_type,
                processor_merchant_id: storage_model
                    .processor_merchant_id
                    .unwrap_or(storage_model.merchant_id),
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                force_3ds_challenge: storage_model.force_3ds_challenge,
                force_3ds_challenge_trigger: storage_model.force_3ds_challenge_trigger,
                is_iframe_redirection_enabled: storage_model.is_iframe_redirection_enabled,
                is_payment_id_from_merchant: storage_model.is_payment_id_from_merchant,
                payment_channel: storage_model.payment_channel,
                tax_status: storage_model.tax_status,
                discount_amount: storage_model.discount_amount,
                shipping_amount_tax: storage_model.shipping_amount_tax,
                duty_amount: storage_model.duty_amount,
                order_date: storage_model.order_date,
                enable_partial_authorization: storage_model.enable_partial_authorization,
                enable_overcapture: storage_model.enable_overcapture,
                mit_category: storage_model.mit_category,
                billing_descriptor: storage_model.billing_descriptor,
                tokenization: storage_model.tokenization,
                partner_merchant_identifier_details: storage_model
                    .partner_merchant_identifier_details,
                state_metadata: storage_model.state_metadata,
                installment_options: storage_model.installment_options.map(|o| o.0),
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment intent".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::PaymentIntentNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: None, // deprecated
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: None,
            split_payments: self.split_payments,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
            request_extended_authorization: self.request_extended_authorization,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            platform_merchant_id: None,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            force_3ds_challenge: self.force_3ds_challenge,
            force_3ds_challenge_trigger: self.force_3ds_challenge_trigger,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            extended_return_url: self.return_url,
            is_payment_id_from_merchant: self.is_payment_id_from_merchant,
            payment_channel: self.payment_channel,
            tax_status: self.tax_status,
            discount_amount: self.discount_amount,
            order_date: self.order_date,
            shipping_amount_tax: self.shipping_amount_tax,
            duty_amount: self.duty_amount,
            enable_partial_authorization: self.enable_partial_authorization,
            enable_overcapture: self.enable_overcapture,
            mit_category: self.mit_category,
            billing_descriptor: self.billing_descriptor,
            tokenization: self.tokenization,
            partner_merchant_identifier_details: self.partner_merchant_identifier_details,
            state_metadata: self.state_metadata,
            installment_options: self
                .installment_options
                .map(common_types::payments::InstallmentOptions),
        })
    }
}
