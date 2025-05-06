use common_utils::{encryption::Encryption, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::enums as storage_enums;
#[cfg(feature = "v1")]
use crate::schema::merchant_account;
#[cfg(feature = "v2")]
use crate::schema_v2::merchant_account;

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the fields read / written will be interchanged
#[cfg(feature = "v1")]
#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    Identifiable,
    serde::Serialize,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_account, primary_key(merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantAccount {
    merchant_id: common_utils::id_type::MerchantId,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub webhook_details: Option<crate::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub intent_fulfillment_time: Option<i64>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<common_utils::id_type::ProfileId>,
    pub recon_status: storage_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub id: Option<common_utils::id_type::MerchantId>,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: Option<common_enums::MerchantAccountType>,
}

#[cfg(feature = "v1")]
pub struct MerchantAccountSetter {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub webhook_details: Option<crate::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub intent_fulfillment_time: Option<i64>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<common_utils::id_type::ProfileId>,
    pub recon_status: storage_enums::ReconStatus,
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
            id: Some(item.merchant_id.clone()),
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
            intent_fulfillment_time: item.intent_fulfillment_time,
            created_at: item.created_at,
            modified_at: item.modified_at,
            frm_routing_algorithm: item.frm_routing_algorithm,
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
            merchant_account_type: Some(item.merchant_account_type),
        }
    }
}

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the fields read / written will be interchanged
#[cfg(feature = "v2")]
#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    Identifiable,
    serde::Serialize,
    Queryable,
    router_derive::DebugAsDisplay,
    Selectable,
)]
#[diesel(table_name = merchant_account, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantAccount {
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: storage_enums::ReconStatus,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub id: common_utils::id_type::MerchantId,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: Option<common_enums::MerchantAccountType>,
}

#[cfg(feature = "v2")]
impl From<MerchantAccountSetter> for MerchantAccount {
    fn from(item: MerchantAccountSetter) -> Self {
        Self {
            id: item.id,
            merchant_name: item.merchant_name,
            merchant_details: item.merchant_details,
            publishable_key: item.publishable_key,
            storage_scheme: item.storage_scheme,
            metadata: item.metadata,
            created_at: item.created_at,
            modified_at: item.modified_at,
            organization_id: item.organization_id,
            recon_status: item.recon_status,
            version: item.version,
            is_platform_account: item.is_platform_account,
            product_type: item.product_type,
            merchant_account_type: Some(item.merchant_account_type),
        }
    }
}

#[cfg(feature = "v2")]
pub struct MerchantAccountSetter {
    pub id: common_utils::id_type::MerchantId,
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: storage_enums::ReconStatus,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
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
    pub fn get_id(&self) -> &common_utils::id_type::MerchantId {
        &self.id
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub return_url: Option<String>,
    pub webhook_details: Option<crate::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub publishable_key: Option<String>,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub intent_fulfillment_time: Option<i64>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub is_recon_enabled: bool,
    pub default_profile: Option<common_utils::id_type::ProfileId>,
    pub recon_status: storage_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub id: Option<common_utils::id_type::MerchantId>,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub publishable_key: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub recon_status: storage_enums::ReconStatus,
    pub id: common_utils::id_type::MerchantId,
    pub version: common_enums::ApiVersion,
    pub is_platform_account: bool,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub merchant_account_type: common_enums::MerchantAccountType,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub publishable_key: Option<String>,
    pub storage_scheme: Option<storage_enums::MerchantStorageScheme>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: time::PrimitiveDateTime,
    pub organization_id: Option<common_utils::id_type::OrganizationId>,
    pub recon_status: Option<storage_enums::ReconStatus>,
    pub is_platform_account: Option<bool>,
    pub product_type: Option<common_enums::MerchantProductType>,
}

#[cfg(feature = "v2")]
impl MerchantAccountUpdateInternal {
    pub fn apply_changeset(self, source: MerchantAccount) -> MerchantAccount {
        let Self {
            merchant_name,
            merchant_details,
            publishable_key,
            storage_scheme,
            metadata,
            modified_at,
            organization_id,
            recon_status,
            is_platform_account,
            product_type,
        } = self;

        MerchantAccount {
            merchant_name: merchant_name.or(source.merchant_name),
            merchant_details: merchant_details.or(source.merchant_details),
            publishable_key: publishable_key.or(source.publishable_key),
            storage_scheme: storage_scheme.unwrap_or(source.storage_scheme),
            metadata: metadata.or(source.metadata),
            created_at: source.created_at,
            modified_at,
            organization_id: organization_id.unwrap_or(source.organization_id),
            recon_status: recon_status.unwrap_or(source.recon_status),
            version: source.version,
            id: source.id,
            is_platform_account: is_platform_account.unwrap_or(source.is_platform_account),
            product_type: product_type.or(source.product_type),
            merchant_account_type: source.merchant_account_type,
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub return_url: Option<String>,
    pub webhook_details: Option<crate::business_profile::WebhookDetails>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub publishable_key: Option<String>,
    pub storage_scheme: Option<storage_enums::MerchantStorageScheme>,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: Option<serde_json::Value>,
    pub modified_at: time::PrimitiveDateTime,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: Option<common_utils::id_type::OrganizationId>,
    pub is_recon_enabled: Option<bool>,
    pub default_profile: Option<Option<common_utils::id_type::ProfileId>>,
    pub recon_status: Option<storage_enums::ReconStatus>,
    pub payment_link_config: Option<serde_json::Value>,
    pub pm_collect_link_config: Option<serde_json::Value>,
    pub is_platform_account: Option<bool>,
    pub product_type: Option<common_enums::MerchantProductType>,
}

#[cfg(feature = "v1")]
impl MerchantAccountUpdateInternal {
    pub fn apply_changeset(self, source: MerchantAccount) -> MerchantAccount {
        let Self {
            merchant_name,
            merchant_details,
            return_url,
            webhook_details,
            sub_merchants_enabled,
            parent_merchant_id,
            enable_payment_response_hash,
            payment_response_hash_key,
            redirect_to_merchant_with_http_post,
            publishable_key,
            storage_scheme,
            locker_id,
            metadata,
            routing_algorithm,
            primary_business_details,
            modified_at,
            intent_fulfillment_time,
            frm_routing_algorithm,
            payout_routing_algorithm,
            organization_id,
            is_recon_enabled,
            default_profile,
            recon_status,
            payment_link_config,
            pm_collect_link_config,
            is_platform_account,
            product_type,
        } = self;

        MerchantAccount {
            merchant_id: source.merchant_id,
            return_url: return_url.or(source.return_url),
            enable_payment_response_hash: enable_payment_response_hash
                .unwrap_or(source.enable_payment_response_hash),
            payment_response_hash_key: payment_response_hash_key
                .or(source.payment_response_hash_key),
            redirect_to_merchant_with_http_post: redirect_to_merchant_with_http_post
                .unwrap_or(source.redirect_to_merchant_with_http_post),
            merchant_name: merchant_name.or(source.merchant_name),
            merchant_details: merchant_details.or(source.merchant_details),
            webhook_details: webhook_details.or(source.webhook_details),
            sub_merchants_enabled: sub_merchants_enabled.or(source.sub_merchants_enabled),
            parent_merchant_id: parent_merchant_id.or(source.parent_merchant_id),
            publishable_key: publishable_key.or(source.publishable_key),
            storage_scheme: storage_scheme.unwrap_or(source.storage_scheme),
            locker_id: locker_id.or(source.locker_id),
            metadata: metadata.or(source.metadata),
            routing_algorithm: routing_algorithm.or(source.routing_algorithm),
            primary_business_details: primary_business_details
                .unwrap_or(source.primary_business_details),
            intent_fulfillment_time: intent_fulfillment_time.or(source.intent_fulfillment_time),
            created_at: source.created_at,
            modified_at,
            frm_routing_algorithm: frm_routing_algorithm.or(source.frm_routing_algorithm),
            payout_routing_algorithm: payout_routing_algorithm.or(source.payout_routing_algorithm),
            organization_id: organization_id.unwrap_or(source.organization_id),
            is_recon_enabled: is_recon_enabled.unwrap_or(source.is_recon_enabled),
            default_profile: default_profile.unwrap_or(source.default_profile),
            recon_status: recon_status.unwrap_or(source.recon_status),
            payment_link_config: payment_link_config.or(source.payment_link_config),
            pm_collect_link_config: pm_collect_link_config.or(source.pm_collect_link_config),
            version: source.version,
            is_platform_account: is_platform_account.unwrap_or(source.is_platform_account),
            id: source.id,
            product_type: product_type.or(source.product_type),
            merchant_account_type: source.merchant_account_type,
        }
    }
}
