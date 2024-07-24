#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
use common_utils::id_type;
#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
use common_utils::id_type;
use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    pii,
    types::keymanager,
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
    pub merchant_id: String,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
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
    pub organization_id: id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<String>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
}

#[cfg(all(feature = "v2", feature = "merchant_account_v2"))]
#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    pub merchant_id: String,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: OptionalEncryptableName,
    pub merchant_details: OptionalEncryptableValue,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
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
    pub organization_id: id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<String>,
    pub recon_status: diesel_models::enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: OptionalEncryptableName,
        merchant_details: OptionalEncryptableValue,
        return_url: Option<String>,
        webhook_details: Option<serde_json::Value>,
        sub_merchants_enabled: Option<bool>,
        parent_merchant_id: Option<String>,
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

impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        let now = date_time::now();

        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_name,
                merchant_details,
                return_url,
                webhook_details,
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
                return_url,
                webhook_details,
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
                modified_at: Some(now),
                intent_fulfillment_time,
                payout_routing_algorithm,
                default_profile,
                payment_link_config,
                pm_collect_link_config,
                ..Default::default()
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                modified_at: Some(now),
                ..Default::default()
            },
            MerchantAccountUpdate::ReconUpdate { recon_status } => Self {
                recon_status: Some(recon_status),
                modified_at: Some(now),
                ..Default::default()
            },
            MerchantAccountUpdate::UnsetDefaultProfile => Self {
                default_profile: Some(None),
                modified_at: Some(now),
                ..Default::default()
            },
            MerchantAccountUpdate::ModifiedAtUpdate => Self {
                modified_at: Some(date_time::now()),
                ..Default::default()
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
        Ok(diesel_models::merchant_account::MerchantAccount {
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
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_store_ref_id: String,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let publishable_key =
            item.publishable_key
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "publishable_key".to_string(),
                })?;

        let identifier = keymanager::Identifier::Merchant(key_store_ref_id.clone());

        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                merchant_id: item.merchant_id,
                return_url: item.return_url,
                enable_payment_response_hash: item.enable_payment_response_hash,
                payment_response_hash_key: item.payment_response_hash_key,
                redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
                merchant_name: item
                    .merchant_name
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, identifier.clone(), key.peek())
                    })
                    .await?,
                merchant_details: item
                    .merchant_details
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, identifier.clone(), key.peek())
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_account_v2")
))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantAccount {
    type DstType = diesel_models::merchant_account::MerchantAccount;
    type NewDstType = diesel_models::merchant_account::MerchantAccountNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::merchant_account::MerchantAccount {
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
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_store_ref_id: String,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let publishable_key =
            item.publishable_key
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "publishable_key".to_string(),
                })?;
        let identifier = keymanager::Identifier::Merchant(key_store_ref_id.clone());
        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                merchant_id: item.merchant_id,
                return_url: item.return_url,
                enable_payment_response_hash: item.enable_payment_response_hash,
                payment_response_hash_key: item.payment_response_hash_key,
                redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
                merchant_name: item
                    .merchant_name
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, identifier.clone(), key.peek())
                    })
                    .await?,
                merchant_details: item
                    .merchant_details
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, identifier.clone(), key.peek())
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
