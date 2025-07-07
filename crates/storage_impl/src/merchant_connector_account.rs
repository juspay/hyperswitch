use std::collections::HashMap;

use common_utils::{
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::{keymanager, keymanager::ToEncryptable},
};
use diesel_models::merchant_connector_account::{
    BillingAccountReference as DieselBillingAccountReference,
    MerchantConnectorAccountFeatureMetadata as DieselMerchantConnectorAccountFeatureMetadata,
    RevenueRecoveryMetadata as DieselRevenueRecoveryMetadata,
};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountFeatureMetadata;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountUpdate;
use hyperswitch_domain_models::{
    business_profile::ProfileSetter,
    merchant_connector_account::{
        AccountReferenceMap, EncryptedMerchantConnectorAccount, MerchantConnectorAccount,
        RevenueRecoveryMetadata,
    },
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use masking::{PeekInterface, Secret};

use crate::utils::ForeignFrom;

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl crate::behaviour::Conversion for MerchantConnectorAccount {
    type DstType = diesel_models::merchant_connector_account::MerchantConnectorAccount;
    type NewDstType = diesel_models::merchant_connector_account::MerchantConnectorAccountNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(
            diesel_models::merchant_connector_account::MerchantConnectorAccount {
                id: self.id,
                merchant_id: self.merchant_id,
                connector_name: self.connector_name,
                connector_account_details: self.connector_account_details.into(),
                disabled: self.disabled,
                payment_methods_enabled: self.payment_methods_enabled,
                connector_type: self.connector_type,
                metadata: self.metadata,
                frm_config: self.frm_configs,
                connector_label: self.connector_label,
                created_at: self.created_at,
                modified_at: self.modified_at,
                connector_webhook_details: self.connector_webhook_details,
                profile_id: self.profile_id,
                applepay_verified_domains: self.applepay_verified_domains,
                pm_auth_config: self.pm_auth_config,
                status: self.status,
                connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
                additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
                version: self.version,
                feature_metadata: self.feature_metadata.map(ForeignFrom::foreign_from),
            },
        )
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let identifier = keymanager::Identifier::Merchant(other.merchant_id.clone());

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
            id: other.id,
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: decrypted_data.connector_account_details,
            disabled: other.disabled,
            payment_methods_enabled: other.payment_methods_enabled,
            connector_type: other.connector_type,
            metadata: other.metadata,

            frm_configs: other.frm_config,
            connector_label: other.connector_label,
            created_at: other.created_at,
            modified_at: other.modified_at,
            connector_webhook_details: other.connector_webhook_details,
            profile_id: other.profile_id,
            applepay_verified_domains: other.applepay_verified_domains,
            pm_auth_config: other.pm_auth_config,
            status: other.status,
            connector_wallets_details: decrypted_data.connector_wallets_details,
            additional_merchant_data: decrypted_data.additional_merchant_data,
            version: other.version,
            feature_metadata: other.feature_metadata.map(ForeignFrom::foreign_from),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(Self::NewDstType {
            id: self.id,
            merchant_id: Some(self.merchant_id),
            connector_name: Some(self.connector_name),
            connector_account_details: Some(self.connector_account_details.into()),
            disabled: self.disabled,
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
            frm_config: self.frm_configs,
            connector_label: self.connector_label,
            created_at: now,
            modified_at: now,
            connector_webhook_details: self.connector_webhook_details,
            profile_id: self.profile_id,
            applepay_verified_domains: self.applepay_verified_domains,
            pm_auth_config: self.pm_auth_config,
            status: self.status,
            connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
            version: self.version,
            feature_metadata: self.feature_metadata.map(ForeignFrom::foreign_from),
        })
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<MerchantConnectorAccountFeatureMetadata>
    for DieselMerchantConnectorAccountFeatureMetadata
{
    fn foreign_from(feature_metadata: MerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            DieselRevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata
                    .billing_connector_retry_threshold,
                billing_account_reference: DieselBillingAccountReference(
                    recovery_metadata.mca_reference.recovery_to_billing,
                ),
            }
        });
        Self { revenue_recovery }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<DieselMerchantConnectorAccountFeatureMetadata>
    for MerchantConnectorAccountFeatureMetadata
{
    fn foreign_from(feature_metadata: DieselMerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            let mut billing_to_recovery = HashMap::new();
            for (key, value) in &recovery_metadata.billing_account_reference.0 {
                billing_to_recovery.insert(value.to_string(), key.clone());
            }
            RevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata
                    .billing_connector_retry_threshold,
                mca_reference: AccountReferenceMap {
                    recovery_to_billing: recovery_metadata.billing_account_reference.0,
                    billing_to_recovery,
                },
            }
        });
        Self { revenue_recovery }
    }
}
