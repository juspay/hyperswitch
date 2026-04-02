use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    pii, type_name,
    types::keymanager::{self},
};
use common_enums::MerchantStorageScheme;
use common_types::business_profile_types::WebhookDetails;
use error_stack::ResultExt;
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::logger;

use crate::{
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
    pub webhook_details: Option<WebhookDetails>,
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
    pub recon_status: common_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
    pub network_tokenization_credentials: OptionalEncryptableValue,
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
    pub webhook_details: Option<WebhookDetails>,
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
    pub recon_status: common_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
    pub network_tokenization_credentials: OptionalEncryptableValue,
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
            network_tokenization_credentials: item.network_tokenization_credentials,
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
    pub recon_status: common_enums::ReconStatus,
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
    pub recon_status: common_enums::ReconStatus,
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

    #[cfg(feature = "v1")]
    /// Get the unique identifier of MerchantAccount
    pub fn get_default_profile(&self) -> &Option<common_utils::id_type::ProfileId> {
        &self.default_profile
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
        webhook_details: Option<WebhookDetails>,
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
        network_tokenization_credentials: OptionalEncryptableValue,
    },
    StorageSchemeUpdate {
        storage_scheme: MerchantStorageScheme,
    },
    ReconUpdate {
        recon_status: common_enums::ReconStatus,
    },
    UnsetDefaultProfile,
    ModifiedAtUpdate,
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
        recon_status: common_enums::ReconStatus,
    },
    ModifiedAtUpdate,
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
