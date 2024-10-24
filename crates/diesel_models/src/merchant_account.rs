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
    pub id: common_utils::id_type::MerchantId,
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

// Edits self based on other value
// Edits other value based on self
trait Edit<T> {
    fn edit_self_from_other(&mut self, other: Self);
    fn edit_value_from_self(self, value:&mut T);
}

// Option<T> implements Edit for two scenarios.
// self reference, points to other if other is Some variant
// value reference, points to internal value of Option if present. 
impl<T> Edit<T> for Option<T> {
    fn edit_self_from_other(&mut self, other: Self) {
        if other.is_some() {
            *self = other;
        }
    }
    fn edit_value_from_self(self, value:&mut T){
        if let Some(x)=self{
            *value=x;
        }
    }
}

impl MerchantAccount {
    #[cfg(feature = "v1")]
    pub fn from_update(&mut self, update: MerchantAccountUpdateInternal) {
        self.merchant_name.edit_self_from_other(update.merchant_name);
        self.merchant_details.edit_self_from_other(update.merchant_details);
        self.return_url.edit_self_from_other(update.return_url);
        self.webhook_details.edit_self_from_other(update.webhook_details);
        self.sub_merchants_enabled.edit_self_from_other(update.sub_merchants_enabled);
        self.parent_merchant_id.edit_self_from_other(update.parent_merchant_id);
        update.enable_payment_response_hash.edit_value_from_self(&mut self.enable_payment_response_hash);
        self.payment_response_hash_key.edit_self_from_other(update.payment_response_hash_key);
        update.redirect_to_merchant_with_http_post.edit_value_from_self(&mut self.redirect_to_merchant_with_http_post);
        self.publishable_key.edit_self_from_other(update.publishable_key);
        update.storage_scheme.edit_value_from_self(&mut self.storage_scheme);
        self.locker_id.edit_self_from_other(update.locker_id);
        self.metadata.edit_self_from_other(update.metadata);
        self.routing_algorithm.edit_self_from_other(update.routing_algorithm);
        update.primary_business_details.edit_value_from_self(&mut self.primary_business_details);
        self.modified_at = update.modified_at;
        self.intent_fulfillment_time.edit_self_from_other(update.intent_fulfillment_time);
        self.frm_routing_algorithm.edit_self_from_other(update.frm_routing_algorithm);
        self.payout_routing_algorithm.edit_self_from_other(update.payout_routing_algorithm);
        update.organization_id.edit_value_from_self(&mut self.organization_id);
        update.is_recon_enabled.edit_value_from_self(&mut self.is_recon_enabled);
        update.default_profile.edit_value_from_self(&mut self.default_profile);
        update.recon_status.edit_value_from_self(&mut self.recon_status);
        self.payment_link_config.edit_self_from_other(update.payment_link_config);
        self.pm_collect_link_config.edit_self_from_other(update.pm_collect_link_config);
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
}
