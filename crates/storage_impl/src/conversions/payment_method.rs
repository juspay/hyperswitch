//! Conversion implementations for PaymentMethod (v1)

use common_utils::{
    errors::{CustomResult, ValidationError},
    pii,
    type_name,
    types::{keymanager, CreatedBy},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_methods::{PaymentMethod, PaymentMethodVaultSourceDetails, StoragePaymentMethodUpdate},
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;
use crate::transformers::ForeignFrom;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        Ok(Self::DstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
            locker_fingerprint_id: self.locker_fingerprint_id,
            network_tokenization_data: self.network_tokenization_data.map(|val| val.into()),
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // Decrypt encrypted fields first
        let (
            payment_method_data,
            payment_method_billing_address,
            network_token_payment_method_data,
            network_tokenization_data,
            customer_details,
        ) = async {
            let payment_method_data = item
                .payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let payment_method_billing_address = item
                .payment_method_billing_address
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_token_payment_method_data = item
                .network_token_payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_tokenization_data = item
                .network_tokenization_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let customer_details = item
                .customer_details
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            Ok::<_, error_stack::Report<common_utils::errors::CryptoError>>(
                (
                    payment_method_data,
                    payment_method_billing_address,
                    network_token_payment_method_data,
                    network_tokenization_data,
                    customer_details,
                ),
            )
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })?;

        let vault_source_details = PaymentMethodVaultSourceDetails::try_from((
            item.vault_type,
            item.external_vault_source,
        ))?;

        // Construct the domain type
        Ok(Self {
            customer_id: item.customer_id,
            merchant_id: item.merchant_id,
            payment_method_id: item.payment_method_id,
            accepted_currency: item.accepted_currency,
            scheme: item.scheme,
            token: item.token,
            cardholder_name: item.cardholder_name,
            issuer_name: item.issuer_name,
            issuer_country: item.issuer_country,
            payer_country: item.payer_country,
            is_stored: item.is_stored,
            swift_code: item.swift_code,
            direct_debit_token: item.direct_debit_token,
            created_at: item.created_at,
            last_modified: item.last_modified,
            payment_method: item.payment_method,
            payment_method_type: item.payment_method_type,
            payment_method_issuer: item.payment_method_issuer,
            payment_method_issuer_code: item.payment_method_issuer_code,
            metadata: item.metadata,
            payment_method_data,
            locker_id: item.locker_id,
            last_used_at: item.last_used_at,
            connector_mandate_details: item.connector_mandate_details,
            customer_acceptance: item.customer_acceptance,
            status: item.status,
            network_transaction_id: item.network_transaction_id,
            client_secret: item.client_secret,
            payment_method_billing_address,
            updated_by: item.updated_by,
            version: item.version,
            network_token_requestor_reference_id: item.network_token_requestor_reference_id,
            network_token_locker_id: item.network_token_locker_id,
            network_token_payment_method_data,
            vault_source_details,
            created_by: item
                .created_by
                .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
            last_modified_by: item
                .last_modified_by
                .and_then(|last_modified_by| last_modified_by.parse::<CreatedBy>().ok()),
            customer_details,
            locker_fingerprint_id: item.locker_fingerprint_id,
            network_tokenization_data,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        Ok(Self::NewDstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
            locker_fingerprint_id: self.locker_fingerprint_id,
            network_tokenization_data: self.network_tokenization_data.map(|val| val.into()),
        })
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<StoragePaymentMethodUpdate> for diesel_models::PaymentMethodUpdate {
    fn foreign_from(update: StoragePaymentMethodUpdate) -> Self {
        match update {
            StoragePaymentMethodUpdate::MetadataUpdateAndLastUsed {
                metadata,
                last_used_at,
                last_modified_by,
            } => Self::MetadataUpdateAndLastUsed {
                metadata,
                last_used_at,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::UpdatePaymentMethodDataAndLastUsed {
                payment_method_data,
                scheme,
                last_used_at,
                last_modified_by,
            } => Self::UpdatePaymentMethodDataAndLastUsed {
                payment_method_data,
                scheme,
                last_used_at,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data,
                last_modified_by,
            } => Self::PaymentMethodDataUpdate {
                payment_method_data,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::LastUsedUpdate { last_used_at } => {
                Self::LastUsedUpdate { last_used_at }
            }
            StoragePaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status,
                last_modified_by,
            } => Self::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::StatusUpdate {
                status,
                last_modified_by,
            } => Self::StatusUpdate {
                status,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::AdditionalDataUpdate {
                payment_method_data,
                status,
                locker_id,
                payment_method,
                payment_method_type,
                payment_method_issuer,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                last_modified_by,
                metadata,
                last_used_at,
                connector_mandate_details,
                network_tokenization_data,
            } => Self::AdditionalDataUpdate {
                payment_method_data,
                status,
                locker_id,
                payment_method,
                payment_method_type,
                payment_method_issuer,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                last_modified_by,
                metadata,
                last_used_at,
                connector_mandate_details,
                network_tokenization_data,
            },
            StoragePaymentMethodUpdate::ConnectorMandateDetailsUpdate {
                connector_mandate_details,
                last_modified_by,
            } => Self::ConnectorMandateDetailsUpdate {
                connector_mandate_details,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::NetworkTokenDataUpdate {
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                network_tokenization_data,
                last_modified_by,
            } => Self::NetworkTokenDataUpdate {
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                network_tokenization_data,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::ConnectorNetworkTransactionIdAndMandateDetailsUpdate {
                connector_mandate_details,
                network_transaction_id,
                last_modified_by,
            } => Self::ConnectorNetworkTransactionIdAndMandateDetailsUpdate {
                connector_mandate_details,
                network_transaction_id,
                last_modified_by,
            },
            StoragePaymentMethodUpdate::PaymentMethodBatchUpdate {
                connector_mandate_details,
                network_transaction_id,
                status,
                payment_method_data,
                last_modified_by,
            } => Self::PaymentMethodBatchUpdate {
                connector_mandate_details,
                network_transaction_id,
                status,
                payment_method_data,
                last_modified_by,
            },
        }
    }
}
