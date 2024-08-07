use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    pii,
    types::keymanager::{self},
};
use diesel_models::{
    enums::MerchantStorageScheme, merchant_account::MerchantAccountUpdateInternal,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::logger;

use crate::type_encryption::{decrypt_optional, AsyncLift};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    merchant_id: common_utils::id_type::MerchantId,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub webhook_details: Option<serde_json::Value>,
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
    pub default_profile: Option<String>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
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
    pub webhook_details: Option<serde_json::Value>,
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
    pub default_profile: Option<String>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
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
        }
    }
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
#[derive(Clone)]
/// Set the private fields of merchant account
pub struct MerchantAccountSetter {
    pub id: common_utils::id_type::MerchantId,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: diesel_models::enums::ReconStatus,
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
impl From<MerchantAccountSetter> for MerchantAccount {
    fn from(item: MerchantAccountSetter) -> Self {
        let MerchantAccountSetter {
            id,
            merchant_name,
            merchant_details,
            publishable_key,
            storage_scheme,
            metadata,
            routing_algorithm,
            frm_routing_algorithm,
            created_at,
            modified_at,
            payout_routing_algorithm,
            organization_id,
            recon_status,
        } = item;
        Self {
            id,
            merchant_name,
            merchant_details,
            publishable_key,
            storage_scheme,
            metadata,
            routing_algorithm,
            frm_routing_algorithm,
            created_at,
            modified_at,
            payout_routing_algorithm,
            organization_id,
            recon_status,
        }
    }
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    id: common_utils::id_type::MerchantId,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub publishable_key: String,
    pub storage_scheme: MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: diesel_models::enums::ReconStatus,
}

impl MerchantAccount {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "merchant_account_v2")
    ))]
    /// Get the unique identifier of MerchantAccount
    pub fn get_id(&self) -> &common_utils::id_type::MerchantId {
        &self.merchant_id
    }

    #[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
    /// Get the unique identifier of MerchantAccount
    pub fn get_id(&self) -> &common_utils::id_type::MerchantId {
        &self.id
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: OptionalEncryptableName,
        merchant_details: OptionalEncryptableValue,
        return_url: Option<String>,
        webhook_details: Option<serde_json::Value>,
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
        default_profile: Option<Option<String>>,
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
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: OptionalEncryptableName,
        merchant_details: OptionalEncryptableValue,
        publishable_key: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        routing_algorithm: Option<serde_json::Value>,
        frm_routing_algorithm: Option<serde_json::Value>,
        payout_routing_algorithm: Option<serde_json::Value>,
    },
    StorageSchemeUpdate {
        storage_scheme: MerchantStorageScheme,
    },
    ReconUpdate {
        recon_status: diesel_models::enums::ReconStatus,
    },
    ModifiedAtUpdate,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]

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
            },
        }
    }
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        let now = date_time::now();

        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_name,
                merchant_details,
                routing_algorithm,
                publishable_key,
                metadata,
                frm_routing_algorithm,
                payout_routing_algorithm,
            } => Self {
                merchant_name: merchant_name.map(Encryption::from),
                merchant_details: merchant_details.map(Encryption::from),
                frm_routing_algorithm,
                routing_algorithm,
                publishable_key,
                metadata,
                modified_at: now,
                payout_routing_algorithm,
                storage_scheme: None,
                organization_id: None,
                recon_status: None,
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                metadata: None,
                routing_algorithm: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                recon_status: None,
            },
            MerchantAccountUpdate::ReconUpdate { recon_status } => Self {
                recon_status: Some(recon_status),
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                storage_scheme: None,
                metadata: None,
                routing_algorithm: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
            },
            MerchantAccountUpdate::ModifiedAtUpdate => Self {
                modified_at: now,
                merchant_name: None,
                merchant_details: None,
                publishable_key: None,
                storage_scheme: None,
                metadata: None,
                routing_algorithm: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                organization_id: None,
                recon_status: None,
            },
        }
    }
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
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
            routing_algorithm: self.routing_algorithm,
            created_at: self.created_at,
            modified_at: self.modified_at,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            organization_id: self.organization_id,
            recon_status: self.recon_status,
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                merchant_details: item
                    .merchant_details
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                publishable_key,
                storage_scheme: item.storage_scheme,
                metadata: item.metadata,
                routing_algorithm: item.routing_algorithm,
                frm_routing_algorithm: item.frm_routing_algorithm,
                created_at: item.created_at,
                modified_at: item.modified_at,
                payout_routing_algorithm: item.payout_routing_algorithm,
                organization_id: item.organization_id,
                recon_status: item.recon_status,
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
            routing_algorithm: self.routing_algorithm,
            created_at: now,
            modified_at: now,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            organization_id: self.organization_id,
            recon_status: self.recon_status,
        })
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantAccount {
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                merchant_details: item
                    .merchant_details
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
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
