use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii, type_name,
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_account::{MerchantAccount, MerchantAccountSetter, MerchantAccountUpdate},
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;
use crate::transformers::ForeignFrom;

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for MerchantAccount {
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
            version: common_types::consts::API_VERSION,
            is_platform_account: self.is_platform_account,
            product_type: self.product_type,
            merchant_account_type: self.merchant_account_type,
        };

        Ok(diesel_models::MerchantAccount::from(setter))
    }

    async fn convert_back(
        state: &KeyManagerState,
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
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
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
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting merchant data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(diesel_models::merchant_account::MerchantAccountNew {
            id: self.id,
            merchant_name: self.merchant_name.map(Encryption::from),
            merchant_details: self.merchant_details.map(Encryption::from),
            publishable_key: Some(self.publishable_key),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            organization_id: self.organization_id,
            recon_status: self.recon_status,
            version: common_types::consts::API_VERSION,
            is_platform_account: self.is_platform_account,
            product_type: self
                .product_type
                .or(Some(common_enums::MerchantProductType::Orchestration)),
            merchant_account_type: self.merchant_account_type,
        })
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for MerchantAccount {
    type DstType = diesel_models::merchant_account::MerchantAccount;
    type NewDstType = diesel_models::merchant_account::MerchantAccountNew;
    
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let setter = diesel_models::merchant_account::MerchantAccountSetter {
            merchant_id: self.get_id().clone(),
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            merchant_name: self.merchant_name.map(|name| name.into()),
            merchant_details: self.merchant_details.map(|details| details.into()),
            webhook_details: self.webhook_details,
            sub_merchants_enabled: self.sub_merchants_enabled,
            parent_merchant_id: self.parent_merchant_id,
            publishable_key: Some(self.publishable_key),
            storage_scheme: self.storage_scheme,
            locker_id: self.locker_id,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            primary_business_details: self.primary_business_details,
            created_at: self.created_at,
            modified_at: self.modified_at,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            organization_id: self.organization_id,
            is_recon_enabled: self.is_recon_enabled,
            default_profile: self.default_profile,
            recon_status: self.recon_status,
            payment_link_config: self.payment_link_config,
            pm_collect_link_config: self.pm_collect_link_config,
            version: self.version,
            is_platform_account: self.is_platform_account,
            product_type: self.product_type,
            merchant_account_type: self.merchant_account_type,
            network_tokenization_credentials: self
                .network_tokenization_credentials
                .map(|credentials| credentials.into()),
        };

        Ok(diesel_models::MerchantAccount::from(setter))
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let merchant_id = item.get_id().to_owned();
        let publishable_key =
            item.publishable_key
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "publishable_key".to_string(),
                })?;

        async {
            let merchant_name = item
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
                .await?;
            let merchant_details = item
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
                .await?;
            let network_tokenization_credentials = item
                .network_tokenization_credentials
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
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(
                MerchantAccountSetter {
                    merchant_id,
                    return_url: item.return_url,
                    enable_payment_response_hash: item.enable_payment_response_hash,
                    payment_response_hash_key: item.payment_response_hash_key,
                    redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
                    merchant_name,
                    merchant_details,
                    webhook_details: item.webhook_details,
                    sub_merchants_enabled: item.sub_merchants_enabled,
                    parent_merchant_id: item.parent_merchant_id,
                    publishable_key,
                    storage_scheme: item.storage_scheme,
                    locker_id: item.locker_id,
                    metadata: item.metadata,
                    routing_algorithm: item.routing_algorithm,
                    frm_routing_algorithm: item.frm_routing_algorithm,
                    primary_business_details: item.primary_business_details,
                    created_at: item.created_at,
                    modified_at: item.modified_at,
                    intent_fulfillment_time: item.intent_fulfillment_time,
                    payout_routing_algorithm: item.payout_routing_algorithm,
                    organization_id: item.organization_id,
                    is_recon_enabled: item.is_recon_enabled,
                    default_profile: item.default_profile,
                    recon_status: item.recon_status,
                    payment_link_config: item.payment_link_config,
                    pm_collect_link_config: item.pm_collect_link_config,
                    version: item.version,
                    is_platform_account: item.is_platform_account,
                    product_type: item.product_type,
                    merchant_account_type: item.merchant_account_type.unwrap_or_default(),
                    network_tokenization_credentials,
                }.into()
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
            id: Some(self.get_id().clone()),
            merchant_id: self.get_id().clone(),
            merchant_name: self.merchant_name.map(Encryption::from),
            merchant_details: self.merchant_details.map(Encryption::from),
            return_url: self.return_url,
            webhook_details: self.webhook_details,
            sub_merchants_enabled: self.sub_merchants_enabled,
            parent_merchant_id: self.parent_merchant_id,
            enable_payment_response_hash: Some(self.enable_payment_response_hash),
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: Some(self.redirect_to_merchant_with_http_post),
            publishable_key: Some(self.publishable_key),
            locker_id: self.locker_id,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            primary_business_details: self.primary_business_details,
            created_at: now,
            modified_at: now,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            organization_id: self.organization_id,
            is_recon_enabled: self.is_recon_enabled,
            default_profile: self.default_profile,
            recon_status: self.recon_status,
            payment_link_config: self.payment_link_config,
            pm_collect_link_config: self.pm_collect_link_config,
            version: common_types::consts::API_VERSION,
            is_platform_account: self.is_platform_account,
            product_type: self
                .product_type
                .or(Some(common_enums::MerchantProductType::Orchestration)),
            merchant_account_type: self.merchant_account_type,
            network_tokenization_credentials: self
                .network_tokenization_credentials
                .map(Encryption::from),
        })
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<MerchantAccountUpdate> for diesel_models::merchant_account::MerchantAccountUpdateInternal {
    fn foreign_from(merchant_account_update: MerchantAccountUpdate) -> Self {
        let now = date_time::now();

        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_name,
                merchant_details,
                webhook_details,
                return_url,
                routing_algorithm,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
                locker_id,
                metadata,
                primary_business_details,
                intent_fulfillment_time,
                frm_routing_algorithm,
                payout_routing_algorithm,
                default_profile,
                payment_link_config,
                pm_collect_link_config,
                network_tokenization_credentials,
            } => Self {
                merchant_name: merchant_name.map(Encryption::from),
                merchant_details: merchant_details.map(Encryption::from),
                frm_routing_algorithm,
                webhook_details,
                routing_algorithm,
                sub_merchants_enabled,
                parent_merchant_id,
                return_url,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
                locker_id,
                metadata,
                primary_business_details,
                modified_at: now,
                intent_fulfillment_time,
                payout_routing_algorithm,
                default_profile,
                payment_link_config,
                pm_collect_link_config,
                storage_scheme: None,
                organization_id: None,
                is_recon_enabled: None,
                recon_status: None,
                is_platform_account: None,
                product_type: None,
                network_tokenization_credentials: network_tokenization_credentials
                    .map(Encryption::from),
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                return_url: None,
                webhook_details: None,
                sub_merchants_enabled: None,
                parent_merchant_id: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                publishable_key: None,
                locker_id: None,
                metadata: None,
                routing_algorithm: None,
                primary_business_details: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                is_recon_enabled: None,
                default_profile: None,
                recon_status: None,
                payment_link_config: None,
                pm_collect_link_config: None,
                is_platform_account: None,
                product_type: None,
                network_tokenization_credentials: None,
            },
            MerchantAccountUpdate::ReconUpdate { recon_status } => Self {
                recon_status: Some(recon_status),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                return_url: None,
                webhook_details: None,
                sub_merchants_enabled: None,
                parent_merchant_id: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                publishable_key: None,
                storage_scheme: None,
                locker_id: None,
                metadata: None,
                routing_algorithm: None,
                primary_business_details: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                is_recon_enabled: None,
                default_profile: None,
                payment_link_config: None,
                pm_collect_link_config: None,
                is_platform_account: None,
                product_type: None,
                network_tokenization_credentials: None,
            },
            MerchantAccountUpdate::UnsetDefaultProfile => Self {
                default_profile: Some(None),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                return_url: None,
                webhook_details: None,
                sub_merchants_enabled: None,
                parent_merchant_id: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                publishable_key: None,
                storage_scheme: None,
                locker_id: None,
                metadata: None,
                routing_algorithm: None,
                primary_business_details: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                is_recon_enabled: None,
                recon_status: None,
                payment_link_config: None,
                pm_collect_link_config: None,
                is_platform_account: None,
                product_type: None,
                network_tokenization_credentials: None,
            },
            MerchantAccountUpdate::ModifiedAtUpdate => Self {
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                return_url: None,
                webhook_details: None,
                sub_merchants_enabled: None,
                parent_merchant_id: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                publishable_key: None,
                storage_scheme: None,
                locker_id: None,
                metadata: None,
                routing_algorithm: None,
                primary_business_details: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                is_recon_enabled: None,
                default_profile: None,
                recon_status: None,
                payment_link_config: None,
                pm_collect_link_config: None,
                is_platform_account: None,
                product_type: None,
                network_tokenization_credentials: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl From<MerchantAccountUpdate> for diesel_models::merchant_account::MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        let now = date_time::now();

        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_name,
                merchant_details,
                publishable_key,
                metadata,
            } => Self {
                merchant_name: merchant_name.map(Encryption::from),
                merchant_details: merchant_details.map(Encryption::from),
                publishable_key,
                metadata: metadata.map(|metadata| *metadata),
                modified_at: now,
                storage_scheme: None,
                organization_id: None,
                recon_status: None,
                is_platform_account: None,
                product_type: None,
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                metadata: None,
                organization_id: None,
                recon_status: None,
                is_platform_account: None,
                product_type: None,
            },
            MerchantAccountUpdate::ReconUpdate { recon_status } => Self {
                recon_status: Some(recon_status),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                storage_scheme: None,
                metadata: None,
                organization_id: None,
                is_platform_account: None,
                product_type: None,
            },
            MerchantAccountUpdate::ModifiedAtUpdate => Self {
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                storage_scheme: None,
                metadata: None,
                organization_id: None,
                recon_status: None,
                is_platform_account: None,
                product_type: None,
            },
        }
    }
}
