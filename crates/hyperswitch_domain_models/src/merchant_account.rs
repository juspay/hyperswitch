use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    pii, type_name,
    types::keymanager::{self},
};
use diesel_models::{
    enums::MerchantStorageScheme, merchant_account::MerchantAccountUpdateInternal,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::logger;

use crate::{
    behaviour::Conversion,
    merchant_key_store,
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    merchant_id: common_utils::id_type::MerchantId,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub webhook_details: Option<diesel_models::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub intent_fulfillment_time: Option<i64>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<common_utils::id_type::ProfileId>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

#[cfg(feature = "v1")]
#[derive(Clone)]
/// Set the private fields of merchant account
pub struct MerchantAccountSetter {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub webhook_details: Option<diesel_models::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub intent_fulfillment_time: Option<i64>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<common_utils::id_type::ProfileId>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

#[cfg(feature = "v1")]
impl From<MerchantAccountSetter> for MerchantAccount {
    fn from(item: MerchantAccountSetter) -> Self {
        Self {
            merchant_id: item.merchant_id,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            merchant_name: item.merchant_name,
            merchant_details: item.merchant_details,
            webhook_details: item.webhook_details,
            sub_merchants_enabled: item.sub_merchants_enabled,
            parent_merchant_id: item.parent_merchant_id,
            publishable_key: item.publishable_key,
            storage_scheme: item.storage_scheme,
            locker_id: item.locker_id,
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            primary_business_details: item.primary_business_details,
            frm_routing_algorithm: item.frm_routing_algorithm,
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
            merchant_account_type: item.merchant_account_type,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone)]
/// Set the private fields of merchant account
pub struct MerchantAccountSetter {
    pub id: common_utils::id_type::MerchantId,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub is_platform_account: bool,
    pub version: common_enums::ApiVersion,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

#[cfg(feature = "v2")]
impl From<MerchantAccountSetter> for MerchantAccount {
    fn from(item: MerchantAccountSetter) -> Self {
        let MerchantAccountSetter {
            id,
            merchant_name,
            merchant_details,
            publishable_key,
            storage_scheme,
            metadata,
            created_at,
            modified_at,
            organization_id,
            recon_status,
            is_platform_account,
            version,
            product_type,
            merchant_account_type,
        } = item;
        Self {
            id,
            merchant_name,
            merchant_details,
            publishable_key,
            storage_scheme,
            metadata,
            created_at,
            modified_at,
            organization_id,
            recon_status,
            is_platform_account,
            version,
            product_type,
            merchant_account_type,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    id: common_utils::id_type::MerchantId,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub is_platform_account: bool,
    pub version: common_enums::ApiVersion,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

impl MerchantAccount {
    #[cfg(feature = "v1")]
    /// Get the unique identifier of MerchantAccount
    pub fn get_id(&self) -> &common_utils::id_type::MerchantId {
        &self.merchant_id
    }

    #[cfg(feature = "v2")]
    /// Get the unique identifier of MerchantAccount
    pub fn get_id(&self) -> &common_utils::id_type::MerchantId {
        &self.id
    }

    /// Get the organization_id from MerchantAccount
    pub fn get_org_id(&self) -> &common_utils::id_type::OrganizationId {
        &self.organization_id
    }

    /// Get the merchant_details from MerchantAccount
    pub fn get_merchant_details(&self) -> &OptionalEncryptableValue {
        &self.merchant_details
    }

    /// Extract merchant_tax_registration_id from merchant_details
    pub fn get_merchant_tax_registration_id(&self) -> Option<Secret<String>> {
        self.merchant_details.as_ref().and_then(|details| {
            details
                .get_inner()
                .peek()
                .get("merchant_tax_registration_id")
                .and_then(|id| id.as_str().map(|s| Secret::new(s.to_string())))
        })
    }

    /// Check whether the merchant account is a platform account
    pub fn is_platform_account(&self) -> bool {
        matches!(
            self.merchant_account_type,
            common_enums::MerchantAccountType::Platform
        )
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: OptionalEncryptableName,
        merchant_details: OptionalEncryptableValue,
        return_url: Option<String>,
        webhook_details: Option<diesel_models::business_profile::WebhookDetails>,
        sub_merchants_enabled: Option<bool>,
        parent_merchant_id: Option<common_utils::id_type::MerchantId>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        publishable_key: Option<String>,
        locker_id: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        routing_algorithm: Option<serde_json::Value>,
        primary_business_details: Option<serde_json::Value>,
        intent_fulfillment_time: Option<i64>,
        frm_routing_algorithm: Option<serde_json::Value>,
        payout_routing_algorithm: Option<serde_json::Value>,
        default_profile: Option<Option<common_utils::id_type::ProfileId>>,
        payment_link_config: Option<serde_json::Value>,
        pm_collect_link_config: Option<serde_json::Value>,
    },
    StorageSchemeUpdate {
        storage_scheme: MerchantStorageScheme,
    },
    ReconUpdate {
        recon_status: diesel_models::enums::ReconStatus,
    },
    UnsetDefaultProfile,
    ModifiedAtUpdate,
    ToPlatformAccount,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: OptionalEncryptableName,
        merchant_details: OptionalEncryptableValue,
        publishable_key: Option<String>,
        metadata: Option<Box<pii::SecretSerdeValue>>,
    },
    StorageSchemeUpdate {
        storage_scheme: MerchantStorageScheme,
    },
    ReconUpdate {
        recon_status: diesel_models::enums::ReconStatus,
    },
    ModifiedAtUpdate,
    ToPlatformAccount,
}

#[cfg(feature = "v1")]
impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
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
            },
            MerchantAccountUpdate::ToPlatformAccount => Self {
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
                is_platform_account: Some(true),
                product_type: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
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
            MerchantAccountUpdate::ToPlatformAccount => Self {
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                storage_scheme: None,
                metadata: None,
                organization_id: None,
                recon_status: None,
                is_platform_account: Some(true),
                product_type: None,
            },
        }
    }
}

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
            merchant_id: self.merchant_id,
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
        let merchant_id = item.get_id().to_owned();
        let publishable_key =
            item.publishable_key
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "publishable_key".to_string(),
                })?;

        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                merchant_id,
                return_url: item.return_url,
                enable_payment_response_hash: item.enable_payment_response_hash,
                payment_response_hash_key: item.payment_response_hash_key,
                redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
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
            id: Some(self.merchant_id.clone()),
            merchant_id: self.merchant_id,
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
        })
    }
}

impl MerchantAccount {
    pub fn get_compatible_connector(&self) -> Option<api_models::enums::Connector> {
        let metadata: Option<api_models::admin::MerchantAccountMetadata> =
            self.metadata.as_ref().and_then(|meta| {
                meta.clone()
                    .parse_value("MerchantAccountMetadata")
                    .map_err(|err| logger::error!("Failed to deserialize {:?}", err))
                    .ok()
            });
        metadata.and_then(|a| a.compatible_connector)
    }
}

#[async_trait::async_trait]
pub trait MerchantAccountInterface
where
    MerchantAccount: Conversion<
        DstType = diesel_models::merchant_account::MerchantAccount,
        NewDstType = diesel_models::merchant_account::MerchantAccountNew,
    >,
{
    type Error;
    async fn insert_merchant(
        &self,
        merchant_account: MerchantAccount,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<MerchantAccount, Self::Error>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<MerchantAccount, Self::Error>;

    async fn update_all_merchant_account(
        &self,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<usize, Self::Error>;

    async fn update_merchant(
        &self,
        this: MerchantAccount,
        merchant_account: MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<MerchantAccount, Self::Error>;

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_account: MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<MerchantAccount, Self::Error>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<(MerchantAccount, merchant_key_store::MerchantKeyStore), Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        organization_id: &common_utils::id_type::OrganizationId,
    ) -> CustomResult<Vec<MerchantAccount>, Self::Error>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> CustomResult<Vec<MerchantAccount>, Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_merchant_and_org_ids(
        &self,
        limit: u32,
        offset: Option<u32>,
    ) -> CustomResult<
        Vec<(
            common_utils::id_type::MerchantId,
            common_utils::id_type::OrganizationId,
        )>,
        Self::Error,
    >;
}
