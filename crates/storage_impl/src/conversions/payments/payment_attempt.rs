//! Conversion implementations for PaymentAttempt

#[cfg(feature = "v1")]
use common_enums as storage_enums;
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
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_attempt::{
    ConnectorErrorDetails, IssuerErrorDetails, NetworkErrorDetails, PaymentAttemptErrorDetails,
    UnifiedErrorDetails,
};
use hyperswitch_domain_models::{
    payments::payment_attempt::{EncryptedPaymentAttempt, PaymentAttempt},
    router_response_types::RedirectForm,
    type_encryption::{crypto_operation, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;
use crate::transformers::{ForeignFrom, ForeignInto};

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentAttemptErrorDetails> for diesel_models::ErrorDetails {
    fn foreign_from(from: PaymentAttemptErrorDetails) -> Self {
        Self {
            unified_details: from.unified_details.foreign_into(),
            issuer_details: from.issuer_details.foreign_into(),
            connector_details: from.connector_details.foreign_into(),
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::ErrorDetails> for PaymentAttemptErrorDetails {
    fn foreign_from(from: diesel_models::ErrorDetails) -> Self {
        Self {
            unified_details: from.unified_details.foreign_into(),
            issuer_details: from.issuer_details.foreign_into(),
            connector_details: from.connector_details.foreign_into(),
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<UnifiedErrorDetails> for diesel_models::UnifiedErrorDetails {
    fn foreign_from(from: UnifiedErrorDetails) -> Self {
        Self {
            category: from.category,
            message: from.message,
            standardised_code: from.standardised_code,
            description: from.description,
            user_guidance_message: from.user_guidance_message,
            recommended_action: from.recommended_action,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::UnifiedErrorDetails> for UnifiedErrorDetails {
    fn foreign_from(from: diesel_models::UnifiedErrorDetails) -> Self {
        Self {
            category: from.category,
            message: from.message,
            standardised_code: from.standardised_code,
            description: from.description,
            user_guidance_message: from.user_guidance_message,
            recommended_action: from.recommended_action,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<IssuerErrorDetails> for diesel_models::IssuerErrorDetails {
    fn foreign_from(from: IssuerErrorDetails) -> Self {
        Self {
            code: from.code,
            message: from.message,
            network_details: from.network_details.foreign_into(),
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::IssuerErrorDetails> for IssuerErrorDetails {
    fn foreign_from(from: diesel_models::IssuerErrorDetails) -> Self {
        Self {
            code: from.code,
            message: from.message,
            network_details: from.network_details.foreign_into(),
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<NetworkErrorDetails> for diesel_models::NetworkErrorDetails {
    fn foreign_from(from: NetworkErrorDetails) -> Self {
        Self {
            name: from.name,
            advice_code: from.advice_code,
            advice_message: from.advice_message,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::NetworkErrorDetails> for NetworkErrorDetails {
    fn foreign_from(from: diesel_models::NetworkErrorDetails) -> Self {
        Self {
            name: from.name,
            advice_code: from.advice_code,
            advice_message: from.advice_message,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<ConnectorErrorDetails> for diesel_models::ConnectorErrorDetails {
    fn foreign_from(from: ConnectorErrorDetails) -> Self {
        Self {
            code: from.code,
            message: from.message,
            reason: from.reason,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::ConnectorErrorDetails> for ConnectorErrorDetails {
    fn foreign_from(from: diesel_models::ConnectorErrorDetails) -> Self {
        Self {
            code: from.code,
            message: from.message,
            reason: from.reason,
        }
    }
}

#[cfg(feature = "v1")]
impl crate::DataModelExt for hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate {
    type StorageModel = diesel_models::PaymentAttemptUpdate;

    fn to_storage_model(self) -> Self::StorageModel {
        ForeignFrom::foreign_from(self)
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        todo!("from_storage_model not implemented for PaymentAttemptUpdate")
    }
}

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
            mandate_details: self.mandate_details.map(diesel_models::enums::MandateDataType::foreign_from),
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
            mandate_data: self.mandate_data.map(diesel_models::enums::MandateDetails::foreign_from),
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
            error_details: self.error_details.map(|v| v.foreign_into()),
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
                mandate_details: storage_model.mandate_details.map(hyperswitch_domain_models::mandates::MandateDataType::foreign_from),
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
                mandate_data: storage_model.mandate_data.map(hyperswitch_domain_models::mandates::MandateDetails::foreign_from),
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
                error_details: storage_model.error_details.map(|v| v.foreign_into()),
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
            mandate_details: self.mandate_details.map(diesel_models::enums::MandateDataType::foreign_from),
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
            mandate_data: self.mandate_data.map(diesel_models::enums::MandateDetails::foreign_from),
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
            error_details: self.error_details.map(|v| v.foreign_into()),
            retry_type: self.retry_type,
            installment_data: self.installment_data,
        })
    }
}

impl ForeignFrom<RedirectForm> for diesel_models::payment_attempt::RedirectForm {
    fn foreign_from(from: RedirectForm) -> Self {
        match from {
            RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            RedirectForm::Html { html_data } => Self::Html { html_data },
            RedirectForm::BarclaycardAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::BarclaycardAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            RedirectForm::BarclaycardConsumerAuth {
                access_token,
                step_up_url,
            } => Self::BarclaycardConsumerAuth {
                access_token,
                step_up_url,
            },
            RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            RedirectForm::DeutschebankThreeDSChallengeFlow { acs_url, creq } => {
                Self::DeutschebankThreeDSChallengeFlow { acs_url, creq }
            }
            RedirectForm::Payme => Self::Payme,
            RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            },
            RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: common_utils::types::Url::wrap(endpoint),
                method,
                form_fields,
                collection_id,
            },
            RedirectForm::WorldpayxmlRedirectForm { jwt } => Self::WorldpayxmlRedirectForm { jwt },
        }
    }
}

impl ForeignFrom<diesel_models::payment_attempt::RedirectForm> for RedirectForm {
    fn foreign_from(from: diesel_models::payment_attempt::RedirectForm) -> Self {
        match from {
            diesel_models::payment_attempt::RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            diesel_models::payment_attempt::RedirectForm::Html { html_data } => {
                Self::Html { html_data }
            }
            diesel_models::payment_attempt::RedirectForm::BarclaycardAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::BarclaycardAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            diesel_models::payment_attempt::RedirectForm::BarclaycardConsumerAuth {
                access_token,
                step_up_url,
            } => Self::BarclaycardConsumerAuth {
                access_token,
                step_up_url,
            },
            diesel_models::payment_attempt::RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            diesel_models::payment_attempt::RedirectForm::DeutschebankThreeDSChallengeFlow { acs_url, creq } => {
                Self::DeutschebankThreeDSChallengeFlow { acs_url, creq }
            }
            diesel_models::payment_attempt::RedirectForm::Payme => Self::Payme,
            diesel_models::payment_attempt::RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            },
            diesel_models::payment_attempt::RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            diesel_models::payment_attempt::RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            diesel_models::payment_attempt::RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: endpoint.into_inner(),
                method,
                form_fields,
                collection_id,
            },
            diesel_models::payment_attempt::RedirectForm::WorldpayxmlRedirectForm { jwt } => {
                Self::WorldpayxmlRedirectForm { jwt }
            }
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate>
    for diesel_models::PaymentAttemptUpdate
{
    fn foreign_from(
        from: hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate,
    ) -> Self {
        use hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate;
        use crate::transformers::ForeignInto;

        match from {
            PaymentAttemptUpdate::Update {
                net_amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                fingerprint_id,
                payment_method_billing_address_id,
                updated_by,
                network_transaction_id,
                order_tax_amount,
                shipping_cost,
            } => Self::Update {
                amount: net_amount.get_order_amount(),
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount: net_amount.get_surcharge_amount(),
                tax_amount: net_amount.get_tax_on_surcharge(),
                fingerprint_id,
                payment_method_billing_address_id,
                network_transaction_id,
                updated_by,
                order_tax_amount,
                shipping_cost,
            },
            PaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                updated_by,
                surcharge_amount,
                tax_amount,
                merchant_connector_id,
                routing_approach,
                is_stored_credential,
            } => Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id,
                routing_approach: routing_approach.map(|approach| match approach {
                    storage_enums::RoutingApproach::Other(_) => {
                        storage_enums::RoutingApproach::default()
                    }
                    _ => approach,
                }),
                is_stored_credential,
            },
            PaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            } => Self::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            },
            PaymentAttemptUpdate::BlocklistUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => Self::BlocklistUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            PaymentAttemptUpdate::ConnectorMandateDetailUpdate {
                connector_mandate_detail,
                tokenization,
                updated_by,
            } => Self::ConnectorMandateDetailUpdate {
                connector_mandate_detail,
                tokenization,
                updated_by,
            },
            PaymentAttemptUpdate::PaymentMethodDetailsUpdate {
                payment_method_id,
                updated_by,
            } => Self::PaymentMethodDetailsUpdate {
                payment_method_id,
                updated_by,
            },
            PaymentAttemptUpdate::ConfirmUpdate {
                net_amount,
                currency,
                status,
                authentication_type,
                capture_method,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                fingerprint_id,
                updated_by,
                merchant_connector_id: connector_id,
                payment_method_id,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                client_source,
                client_version,
                customer_acceptance,
                installment_data,
                connector_mandate_detail,
                tokenization,
                card_discovery,
                routing_approach,
                connector_request_reference_id,
                network_transaction_id,
                is_stored_credential,
                request_extended_authorization,
            } => Self::ConfirmUpdate {
                amount: net_amount.get_order_amount(),
                currency,
                status,
                authentication_type,
                capture_method,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                surcharge_amount: net_amount.get_surcharge_amount(),
                tax_amount: net_amount.get_tax_on_surcharge(),
                fingerprint_id,
                updated_by,
                merchant_connector_id: connector_id,
                payment_method_id,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                client_source,
                client_version,
                customer_acceptance,
                shipping_cost: net_amount.get_shipping_cost(),
                order_tax_amount: net_amount.get_order_tax_amount(),
                installment_data,
                connector_mandate_detail,
                tokenization,
                card_discovery,
                routing_approach: routing_approach.map(|approach| match approach {
                    storage_enums::RoutingApproach::Other(_) => {
                        storage_enums::RoutingApproach::default()
                    }
                    _ => approach,
                }),
                connector_request_reference_id,
                network_transaction_id,
                is_stored_credential,
                request_extended_authorization,
            },
            PaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            } => Self::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            },
            PaymentAttemptUpdate::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                network_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
                standardised_code,
                description,
                user_guidance_message,
                capture_before,
                extended_authorization_last_applied_at,
                extended_authorization_applied,
                payment_method_data,
                encrypted_payment_method_data,
                connector_mandate_detail,
                tokenization,
                charges,
                setup_future_usage_applied,
                debit_routing_savings: _,
                is_overcapture_enabled,
                authorized_amount,
                issuer_error_code,
                issuer_error_message,
                network_details,
                network_error_message,
                recommended_action,
                card_network,
            } => {
                let connector_details = ConnectorErrorDetails::new(
                    error_code.clone(),
                    error_message.clone(),
                    error_reason.clone(),
                );
                let unified_details = UnifiedErrorDetails::new(
                    unified_code.clone(),
                    unified_message.clone(),
                    standardised_code,
                    description.clone(),
                    user_guidance_message.clone(),
                    recommended_action,
                );
                let network_error_details = NetworkErrorDetails::new(
                    network_details,
                    network_error_message.clone(),
                    card_network,
                );
                let issuer_details = IssuerErrorDetails::new(
                    issuer_error_code.clone(),
                    issuer_error_message.clone(),
                    network_error_details,
                );
                let error_details = Box::new(
                    PaymentAttemptErrorDetails::new(
                        unified_details,
                        issuer_details,
                        connector_details,
                    )
                    .map(|opt| opt.map(ForeignInto::foreign_into)),
                );
                Self::ResponseUpdate {
                    status,
                    connector,
                    connector_transaction_id,
                    authentication_type,
                    payment_method_id,
                    mandate_id,
                    connector_metadata,
                    payment_token,
                    error_code,
                    error_message,
                    error_reason,
                    connector_response_reference_id,
                    amount_capturable,
                    updated_by,
                    authentication_data,
                    encoded_data,
                    unified_code,
                    unified_message,
                    capture_before,
                    extended_authorization_applied,
                    extended_authorization_last_applied_at,
                    payment_method_data,
                    connector_mandate_detail: *connector_mandate_detail,
                    tokenization,
                    charges,
                    setup_future_usage_applied,
                    network_transaction_id,
                    is_overcapture_enabled,
                    authorized_amount,
                    encrypted_payment_method_data: encrypted_payment_method_data
                        .map(Encryption::from),
                    error_details,
                }
            }
            PaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            } => {
                let connector_details = ConnectorErrorDetails::new(
                    error_code.clone(),
                    error_message.clone(),
                    error_reason.clone(),
                );
                let unified_details = UnifiedErrorDetails::new(
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                let issuer_details = IssuerErrorDetails::new(None, None, None);
                let error_details = Box::new(
                    PaymentAttemptErrorDetails::new(
                        unified_details,
                        issuer_details,
                        connector_details,
                    )
                    .map(|opt| opt.map(ForeignInto::foreign_into)),
                );
                Self::UnresolvedResponseUpdate {
                    status,
                    connector,
                    connector_transaction_id,
                    payment_method_id,
                    error_code,
                    error_message,
                    error_reason,
                    connector_response_reference_id,
                    updated_by,
                    error_details,
                }
            }
            PaymentAttemptUpdate::StatusUpdate { status, updated_by } => {
                Self::StatusUpdate { status, updated_by }
            }
            PaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                standardised_code,
                description,
                user_guidance_message,
                connector_transaction_id,
                connector_response_reference_id,
                payment_method_data,
                encrypted_payment_method_data,
                authentication_type,
                issuer_error_code,
                issuer_error_message,
                network_details,
                network_error_message,
                recommended_action,
                card_network,
            } => {
                let connector_details = ConnectorErrorDetails::new(
                    error_code.clone(),
                    error_message.clone(),
                    error_reason.clone(),
                );
                let unified_details = UnifiedErrorDetails::new(
                    unified_code.clone(),
                    unified_message.clone(),
                    standardised_code,
                    description.clone(),
                    user_guidance_message.clone(),
                    recommended_action,
                );
                let network_error_details = NetworkErrorDetails::new(
                    network_details.clone(),
                    network_error_message.clone(),
                    card_network,
                );
                let issuer_details = IssuerErrorDetails::new(
                    issuer_error_code.clone(),
                    issuer_error_message.clone(),
                    network_error_details,
                );
                let error_details = Box::new(
                    PaymentAttemptErrorDetails::new(
                        unified_details,
                        issuer_details,
                        connector_details,
                    )
                    .map(|opt| opt.map(ForeignInto::foreign_into)),
                );
                Self::ErrorUpdate {
                    connector,
                    status,
                    error_code,
                    error_message,
                    error_reason,
                    amount_capturable,
                    updated_by,
                    unified_code,
                    unified_message,
                    connector_transaction_id,
                    connector_response_reference_id,
                    payment_method_data,
                    authentication_type,
                    issuer_error_code,
                    issuer_error_message,
                    network_details,
                    encrypted_payment_method_data: encrypted_payment_method_data
                        .map(Encryption::from),
                    error_details,
                }
            }
            PaymentAttemptUpdate::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            } => Self::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            },
            PaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            } => Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            },
            PaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => Self::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            PaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            } => Self::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            },
            PaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                charges,
                updated_by,
            } => Self::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                charges,
                connector,
                updated_by,
            },
            PaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                net_amount,
                amount_capturable,
            } => Self::IncrementalAuthorizationAmountUpdate {
                amount: net_amount.get_order_amount(),
                amount_capturable,
            },
            PaymentAttemptUpdate::AuthenticationUpdate {
                status,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
            } => Self::AuthenticationUpdate {
                status,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
            },
            PaymentAttemptUpdate::ManualUpdate {
                status,
                error_code,
                error_message,
                error_reason,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
                amount_capturable,
            } => Self::ManualUpdate {
                status,
                error_code,
                error_message,
                error_reason,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
                amount_capturable,
            },
            PaymentAttemptUpdate::PostSessionTokensUpdate {
                updated_by,
                connector_metadata,
            } => Self::PostSessionTokensUpdate {
                updated_by,
                connector_metadata,
            },
        }
    }
}
