use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::StrongSecret;

use crate::{enums as storage_enums, schema::merchant_account};

#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Eq,
    PartialEq,
    Identifiable,
    Queryable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccount {
    pub id: i32,
    pub merchant_id: String,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<String>,
    pub merchant_details: Option<serde_json::Value>,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub api_key: Option<StrongSecret<String>>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_id: String,
    pub merchant_name: Option<String>,
    pub merchant_details: Option<serde_json::Value>,
    pub return_url: Option<String>,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub publishable_key: Option<String>,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: serde_json::Value,
    pub api_key: Option<StrongSecret<String>>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: Option<String>,
        merchant_details: Option<serde_json::Value>,
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
        frm_routing_algorithm: Option<serde_json::Value>,
    },
    StorageSchemeUpdate {
        storage_scheme: storage_enums::MerchantStorageScheme,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
    merchant_name: Option<String>,
    merchant_details: Option<serde_json::Value>,
    return_url: Option<String>,
    webhook_details: Option<serde_json::Value>,
    sub_merchants_enabled: Option<bool>,
    parent_merchant_id: Option<String>,
    enable_payment_response_hash: Option<bool>,
    payment_response_hash_key: Option<String>,
    redirect_to_merchant_with_http_post: Option<bool>,
    publishable_key: Option<String>,
    storage_scheme: Option<storage_enums::MerchantStorageScheme>,
    locker_id: Option<String>,
    metadata: Option<pii::SecretSerdeValue>,
    routing_algorithm: Option<serde_json::Value>,
    primary_business_details: Option<serde_json::Value>,
    modified_at: Option<time::PrimitiveDateTime>,
    frm_routing_algorithm: Option<serde_json::Value>,
}

impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
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
                frm_routing_algorithm,
            } => Self {
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
                frm_routing_algorithm,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                ..Default::default()
            },
        }
    }
}
