use common_utils::{
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::keymanager,
};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_account::MerchantAccountUpdate;
use hyperswitch_domain_models::{
    merchant_account::MerchantAccountSetter,
    merchant_context::MerchantAccount,
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use masking::{PeekInterface, Secret};

#[cfg(feature = "v2")]
use crate::utils::ForeignFrom;

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantAccount {
    type DstType = diesel_models::merchant_account::MerchantAccount;
    type NewDstType = diesel_models::merchant_account::MerchantAccountNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let id = self.get_id().to_owned();

        let setter = diesel_models::merchant_account::MerchantAccountSetter {
            id,
            merchant_name: self.merchant_name.map(|name| name.into()),
            merchant_details: self.merchant_details.map(|details| details.into()),
            publishable_key: Some(self.publishable_key),
            storage_scheme: self.storage_scheme,
            metadata: self.metadata,
            created_at: self.created_at,
            modified_at: self.modified_at,
            organization_id: self.organization_id,
            recon_status: self.recon_status,
            version: common_utils::consts::API_VERSION,
            is_platform_account: self.is_platform_account,
            product_type: self.product_type,
            merchant_account_type: self.merchant_account_type,
        };

        Ok(diesel_models::MerchantAccount::from(setter))
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
        let id = item.get_id().to_owned();
        let publishable_key =
            item.publishable_key
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "publishable_key".to_string(),
                })?;

        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(
                MerchantAccountSetter {
                    id,
                    merchant_name: item
                        .merchant_name
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
                        .await?,
                    merchant_details: item
                        .merchant_details
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
                        .await?,
                    publishable_key,
                    storage_scheme: item.storage_scheme,
                    metadata: item.metadata,
                    created_at: item.created_at,
                    modified_at: item.modified_at,
                    organization_id: item.organization_id,
                    recon_status: item.recon_status,
                    is_platform_account: item.is_platform_account,
                    version: item.version,
                    product_type: item.product_type,
                    merchant_account_type: item.merchant_account_type.unwrap_or_default(),
                }
                .into(),
            )
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting merchant data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(diesel_models::merchant_account::MerchantAccountNew {
            id: self.get_id().clone(),
            merchant_name: self.merchant_name.map(Encryption::from),
            merchant_details: self.merchant_details.map(Encryption::from),
            publishable_key: Some(self.publishable_key),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            organization_id: self.organization_id,
            recon_status: self.recon_status,
            version: common_utils::consts::API_VERSION,
            is_platform_account: self.is_platform_account,
            product_type: self
                .product_type
                .or(Some(common_enums::MerchantProductType::Orchestration)),
            merchant_account_type: self.merchant_account_type,
        })
    }
}
