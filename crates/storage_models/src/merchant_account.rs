use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::StrongSecret;

use crate::{enums as storage_enums, schema::merchant_account};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccount {
    pub id: i32,
    pub merchant_id: String,
    pub api_key: Option<StrongSecret<String>>,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<String>,
    pub merchant_details: Option<serde_json::Value>,
    pub webhook_details: Option<serde_json::Value>,
    pub routing_algorithm: Option<storage_enums::RoutingAlgorithm>,
    pub custom_routing_rules: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub publishable_key: Option<String>,
    pub storage_scheme: storage_enums::MerchantStorageScheme,
}

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_id: String,
    pub merchant_name: Option<String>,
    pub api_key: Option<StrongSecret<String>>,
    pub merchant_details: Option<serde_json::Value>,
    pub return_url: Option<String>,
    pub webhook_details: Option<serde_json::Value>,
    pub routing_algorithm: Option<storage_enums::RoutingAlgorithm>,
    pub custom_routing_rules: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub publishable_key: Option<String>,
}

#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_id: String,
        merchant_name: Option<String>,
        api_key: Option<StrongSecret<String>>,
        merchant_details: Option<serde_json::Value>,
        return_url: Option<String>,
        webhook_details: Option<serde_json::Value>,
        routing_algorithm: Option<storage_enums::RoutingAlgorithm>,
        custom_routing_rules: Option<serde_json::Value>,
        sub_merchants_enabled: Option<bool>,
        parent_merchant_id: Option<String>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        publishable_key: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountUpdateInternal {
    merchant_id: Option<String>,
    merchant_name: Option<String>,
    api_key: Option<StrongSecret<String>>,
    merchant_details: Option<serde_json::Value>,
    return_url: Option<String>,
    webhook_details: Option<serde_json::Value>,
    routing_algorithm: Option<storage_enums::RoutingAlgorithm>,
    custom_routing_rules: Option<serde_json::Value>,
    sub_merchants_enabled: Option<bool>,
    parent_merchant_id: Option<String>,
    enable_payment_response_hash: Option<bool>,
    payment_response_hash_key: Option<String>,
    redirect_to_merchant_with_http_post: Option<bool>,
    publishable_key: Option<String>,
}

impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_id,
                merchant_name,
                api_key,
                merchant_details,
                return_url,
                webhook_details,
                routing_algorithm,
                custom_routing_rules,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
            } => Self {
                merchant_id: Some(merchant_id),
                merchant_name,
                api_key,
                merchant_details,
                return_url,
                webhook_details,
                routing_algorithm,
                custom_routing_rules,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
            },
        }
    }
}
