use common_utils::pii;
use masking::{Secret, StrongSecret};
use storage_models::enums;
use time::PrimitiveDateTime;

use super::behaviour;

#[derive(Clone, Debug)]
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
    pub storage_scheme: enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub api_key: Option<StrongSecret<String>>,
}
