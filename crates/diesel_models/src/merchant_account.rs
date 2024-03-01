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
    pub primary_business_details: serde_json::Value,
    pub intent_fulfillment_time: Option<i64>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: String,
    pub is_recon_enabled: bool,
    pub default_profile: Option<String>,
    pub recon_status: storage_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
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
    pub primary_business_details: serde_json::Value,
    pub intent_fulfillment_time: Option<i64>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: String,
    pub is_recon_enabled: bool,
    pub default_profile: Option<String>,
    pub recon_status: storage_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
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
    pub storage_scheme: Option<storage_enums::MerchantStorageScheme>,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub primary_business_details: Option<serde_json::Value>,
    pub modified_at: Option<time::PrimitiveDateTime>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub organization_id: Option<String>,
    pub is_recon_enabled: bool,
    pub default_profile: Option<Option<String>>,
    pub recon_status: storage_enums::ReconStatus,
    pub payment_link_config: Option<serde_json::Value>,
}
