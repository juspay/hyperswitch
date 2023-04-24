use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::{encryption::Encryption, enums as storage_enums, schema::merchant_account};

#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
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
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub api_key: Option<Encryption>,
    pub primary_business_details: serde_json::Value,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_id: String,
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
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
    pub api_key: Option<Encryption>,
    pub primary_business_details: serde_json::Value,
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
    },
    StorageSchemeUpdate {
        storage_scheme: storage_enums::MerchantStorageScheme,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
    pub merchant_name: Option<Encryption>,
    pub merchant_details: Option<Encryption>,
    pub api_key: Option<Encryption>,
    pub return_url: Option<String>,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub publishable_key: Option<String>,
    pub storage_scheme: Option<storage_enums::MerchantStorageScheme>,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: Option<serde_json::Value>,
    pub modified_at: Option<time::PrimitiveDateTime>,
}
