//! Conversion implementations for MerchantConnectorAccount

use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_connector_account::{
        EncryptedMerchantConnectorAccount, MerchantConnectorAccount, MerchantConnectorAccountUpdate,
    },
    type_encryption::{crypto_operation, CryptoOperation},
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::{
    MerchantConnectorAccountFeatureMetadata, RevenueRecoveryMetadata, AccountReferenceMap,
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;
use crate::transformers::ForeignFrom;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for MerchantConnectorAccount {
    type DstType = diesel_models::merchant_connector_account::MerchantConnectorAccount;
    type NewDstType = diesel_models::merchant_connector_account::MerchantConnectorAccountNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::merchant_connector_account::MerchantConnectorAccount {
            merchant_id: self.merchant_id,
            connector_name: self.connector_name,
            connector_account_details: self.connector_account_details.into(),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self.merchant_connector_id.clone(),
            id: Some(self.merchant_connector_id),
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: self.connector_type,
            metadata: self.metadata,
            frm_configs: None,
            frm_config: self.frm_configs,
            business_country: self.business_country,
            business_label: self.business_label,
            connector_label: self.connector_label,
            business_sub_label: self.business_sub_label,
            created_at: self.created_at,
            modified_at: self.modified_at,
            connector_webhook_details: self.connector_webhook_details,
            profile_id: Some(self.profile_id),
            applepay_verified_domains: self.applepay_verified_domains,
            pm_auth_config: self.pm_auth_config,
            status: self.status,
            connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
            version: self.version,
            connector_webhook_registration_details: self.connector_webhook_registration_details,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let identifier = Identifier::Merchant(other.merchant_id.clone());
        let decrypted_data = crypto_operation(
            state,
            type_name!(Self::DstType),
            CryptoOperation::BatchDecrypt(EncryptedMerchantConnectorAccount::to_encryptable(
                EncryptedMerchantConnectorAccount {
                    connector_account_details: other.connector_account_details,
                    additional_merchant_data: other.additional_merchant_data,
                    connector_wallets_details: other.connector_wallets_details,
                },
            )),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting connector account details".to_string(),
        })?;

        let decrypted_data = EncryptedMerchantConnectorAccount::from_encryptable(decrypted_data)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            })?;

        Ok(Self {
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: decrypted_data.connector_account_details,
            test_mode: other.test_mode,
            disabled: other.disabled,
            merchant_connector_id: other.merchant_connector_id,
            payment_methods_enabled: other.payment_methods_enabled,
            connector_type: other.connector_type,
            metadata: other.metadata,
            frm_configs: other.frm_config,
            business_country: other.business_country,
            business_label: other.business_label,
            connector_label: other.connector_label,
            business_sub_label: other.business_sub_label,
            created_at: other.created_at,
            modified_at: other.modified_at,
            connector_webhook_details: other.connector_webhook_details,
            profile_id: other
                .profile_id
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "profile_id".to_string(),
                })?,
            applepay_verified_domains: other.applepay_verified_domains,
            pm_auth_config: other.pm_auth_config,
            status: other.status,
            connector_wallets_details: decrypted_data.connector_wallets_details,
            additional_merchant_data: decrypted_data.additional_merchant_data,
            version: other.version,
            connector_webhook_registration_details: other.connector_webhook_registration_details,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(Self::NewDstType {
            merchant_id: Some(self.merchant_id),
            connector_name: Some(self.connector_name),
            connector_account_details: Some(self.connector_account_details.into()),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self.merchant_connector_id.clone(),
            id: Some(self.merchant_connector_id),
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
            frm_configs: None,
            frm_config: self.frm_configs,
            business_country: self.business_country,
            business_label: self.business_label,
            connector_label: self.connector_label,
            business_sub_label: self.business_sub_label,
            created_at: now,
            modified_at: now,
            connector_webhook_details: self.connector_webhook_details,
            profile_id: Some(self.profile_id),
            applepay_verified_domains: self.applepay_verified_domains,
            pm_auth_config: self.pm_auth_config,
            status: self.status,
            connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
            version: self.version,
        })
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<MerchantConnectorAccountUpdate>
    for diesel_models::merchant_connector_account::MerchantConnectorAccountUpdateInternal
{
    fn foreign_from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                connector_type,
                connector_name,
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
                frm_configs,
                connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config,
                connector_label,
                status,
                connector_wallets_details,
                additional_merchant_data,
            } => Self {
                connector_type,
                connector_name,
                connector_account_details: connector_account_details.map(Encryption::from),
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
                frm_configs: None,
                frm_config: frm_configs,
                modified_at: Some(date_time::now()),
                connector_webhook_details: *connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config: *pm_auth_config,
                connector_label,
                status,
                connector_wallets_details: connector_wallets_details.map(Encryption::from),
                additional_merchant_data: additional_merchant_data.map(Encryption::from),
                connector_webhook_registration_details: None,
            },
            MerchantConnectorAccountUpdate::ConnectorWalletDetailsUpdate {
                connector_wallets_details,
            } => Self {
                connector_wallets_details: Some(Encryption::from(connector_wallets_details)),
                connector_type: None,
                connector_name: None,
                connector_account_details: None,
                connector_label: None,
                test_mode: None,
                disabled: None,
                merchant_connector_id: None,
                payment_methods_enabled: None,
                frm_configs: None,
                metadata: None,
                modified_at: None,
                connector_webhook_details: None,
                frm_config: None,
                applepay_verified_domains: None,
                pm_auth_config: None,
                status: None,
                additional_merchant_data: None,
                connector_webhook_registration_details: None,
            },
            MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
                connector_webhook_registration_details,
            } => Self {
                connector_type: None,
                connector_name: None,
                connector_account_details: None,
                connector_label: None,
                test_mode: None,
                disabled: None,
                merchant_connector_id: None,
                payment_methods_enabled: None,
                frm_configs: None,
                metadata: None,
                modified_at: None,
                connector_webhook_details: None,
                frm_config: None,
                applepay_verified_domains: None,
                pm_auth_config: None,
                status: None,
                connector_wallets_details: None,
                additional_merchant_data: None,
                connector_webhook_registration_details,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl From<MerchantConnectorAccountUpdate>
    for diesel_models::merchant_connector_account::MerchantConnectorAccountUpdateInternal
{
    fn from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                connector_type,
                connector_account_details,
                disabled,
                payment_methods_enabled,
                metadata,
                frm_configs,
                connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config,
                connector_label,
                status,
                connector_wallets_details,
                additional_merchant_data,
                feature_metadata,
            } => Self {
                connector_type,
                connector_account_details: connector_account_details.map(Encryption::from),
                disabled,
                payment_methods_enabled,
                metadata,
                frm_config: frm_configs,
                modified_at: Some(date_time::now()),
                connector_webhook_details: *connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config: *pm_auth_config,
                connector_label,
                status,
                connector_wallets_details: connector_wallets_details.map(Encryption::from),
                additional_merchant_data: additional_merchant_data.map(Encryption::from),
                feature_metadata: feature_metadata.map(From::from),
            },
            MerchantConnectorAccountUpdate::ConnectorWalletDetailsUpdate {
                connector_wallets_details,
            } => Self {
                connector_wallets_details: Some(Encryption::from(connector_wallets_details)),
                connector_type: None,
                connector_account_details: None,
                connector_label: None,
                disabled: None,
                payment_methods_enabled: None,
                metadata: None,
                modified_at: None,
                connector_webhook_details: None,
                frm_config: None,
                applepay_verified_domains: None,
                pm_auth_config: None,
                status: None,
                additional_merchant_data: None,
                feature_metadata: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl From<MerchantConnectorAccountFeatureMetadata>
    for diesel_models::merchant_connector_account::MerchantConnectorAccountFeatureMetadata
{
    fn from(feature_metadata: MerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            diesel_models::merchant_connector_account::RevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata.billing_connector_retry_threshold,
                billing_account_reference: diesel_models::merchant_connector_account::BillingAccountReference(
                    recovery_metadata.mca_reference.recovery_to_billing,
                ),
            }
        });
        Self { revenue_recovery }
    }
}

#[cfg(feature = "v2")]
impl From<diesel_models::merchant_connector_account::MerchantConnectorAccountFeatureMetadata>
    for MerchantConnectorAccountFeatureMetadata
{
    fn from(
        feature_metadata: diesel_models::merchant_connector_account::MerchantConnectorAccountFeatureMetadata,
    ) -> Self {
        use std::collections::HashMap;
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            let mut billing_to_recovery = HashMap::new();
            for (key, value) in &recovery_metadata.billing_account_reference.0 {
                billing_to_recovery.insert(value.to_string(), key.clone());
            }
            RevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata.billing_connector_retry_threshold,
                mca_reference: AccountReferenceMap {
                    recovery_to_billing: recovery_metadata.billing_account_reference.0,
                    billing_to_recovery,
                },
            }
        });
        Self { revenue_recovery }
    }
}
